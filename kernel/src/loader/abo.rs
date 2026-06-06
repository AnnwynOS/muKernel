//! Loader ABO ; Application Bundle Object amené à devenir le format natif IdealOS
//! Contrairement à ELF ; qui se présente de mon point de vue comme un blob opaque, ABO intère identité via UUID ; manifest déclarant capabilities requises et exposées et sandbox profile en plus du code natif 86_64 et/ou WASM ansi que des métadonnées structurées.
//! Format binaire v0 :
//! Offset/Taille/Description
//!  0/4/Magic : b"ABO\0"
//!  4/2/Version majeure (0 pour v0)
//!  6/2/Version mineure
//!  8/16/UUID du composant (128 bits, RFC 4122)
//! 24/4/Flags (bit 0 = a du code natif, bit 1 = a du WASM)
//! 28/4/Offset du manifest depuis début du fichier
//! 32/4/Taille du manifest en octets
//! 36/4/Offset de la table de segments natifs
//! 40/4/Nombre de segments natifs
//! 44/4/Entry point (offset dans le premier segment exécutable)
//! 48/4/Offset section WASM (0 si absent)
//! 52/4/Taille section WASM
//! 56/8/Réservé (zéro)
//! 64 octets pour le Header
//! 64+/.../Manifest (texte structuré, format propre)
//! .../.../Table de segments (AboSegment × n)
//! .../.../Données des segments
//! .../.../Section WASM (optionnel)
//! Table de segments (par segment, 32 octets) :
//!  0/8/Adresse virtuelle user de destination
//!  8/8/Taille en mémoire
//! 16/8/Offset des données dans le fichier ABO
//! 24/4/Taille des données dans le fichier
//! 28/4/Flags (bit 0=R, 1=W, 2=X)

use x86_64::structures::paging::PageTableFlags;
use crate::process::ProcessId;
use super::{LoadError, LoadedBinary, USER_STACK_BASE, USER_STACK_SIZE};
use super::segments::{Segment, load_segment, map_stack};

pub const ABO_MAGIC:[u8; 4] = *b"ABO\0";
pub const ABO_VERSION_MAJ:u16 = 0;

pub const FLAG_NATIVE: u32 = 1 << 0;
pub const FLAG_WASM: u32 = 1 << 1;

pub const SEG_FLAG_R: u32 = 1 << 0;
pub const SEG_FLAG_W: u32 = 1 << 1;
pub const SEG_FLAG_X: u32 = 1 << 2;

#[repr(C, packed)]
struct AboHeader {
    magic: [u8; 4],
    version_major: u16,
    version_minor: u16,
    uuid: [u8; 16],
    flags: u32,
    manifest_off: u32,
    manifest_size: u32,
    segments_off: u32,
    segments_count: u32,
    entry_offset: u32,
    wasm_off: u32,
    wasm_size: u32,
    _reserved: u64,
}

#[repr(C, packed)]
struct AboSegment {
    vaddr: u64,
    mem_size: u64,
    file_off: u64,
    file_size: u32,
    flags: u32,
}

const HEADER_SIZE:  usize = core::mem::size_of::<AboHeader>();
const SEGMENT_SIZE: usize = core::mem::size_of::<AboSegment>();

// Manifest :
// Le manifest est un format texte minimaliste
// Format : lignes "KEY=VALUE\n"
// Clés reconnues :
// CAP_REQ=<kind>:<rights>  //capability requise
// CAP_EXP=<service_id> //service exposé
// SANDBOX=no_network //restriction sandbox
// NAME=<string>   //nom lisible
// VERSION=<semver> //version du composant

struct Manifest<'a> {
    raw: &'a [u8],
}

impl<'a> Manifest<'a> {
    fn new(raw: &'a [u8]) -> Self { Self { raw } }

    fn cap_required(&self) -> impl Iterator<Item = &[u8]> {
        self.lines_with_prefix(b"CAP_REQ=")
    }

    fn lines_with_prefix(&self, prefix: &'static [u8]) -> impl Iterator<Item = &[u8]> {
        self.raw
            .split(|&b| b == b'\n')
            .filter(move |line| line.starts_with(prefix))
            .map(move |line| &line[prefix.len()..])
    }

    fn has_sandbox(&self, key: &[u8]) -> bool {
        let prefix = b"SANDBOX=";
        self.raw
            .split(|&b| b == b'\n')
            .filter(|line| line.starts_with(prefix))
            .any(|line| &line[prefix.len()..] == key)
    }
}

pub fn load(pid: ProcessId, data: &[u8]) -> Result<LoadedBinary, LoadError> {
    // validation du header
    let hdr = parse_header(data)?;

    let flags = { hdr.flags };
    let manifest_off= { hdr.manifest_off } as usize;
    let manifest_size= { hdr.manifest_size } as usize;
    let segments_off= { hdr.segments_off } as usize;
    let segments_count= { hdr.segments_count } as usize;
    let entry_offset= { hdr.entry_offset } as u64;

    // parsing du manifest
    let manifest = extract_manifest(data, manifest_off, manifest_size)?;

    // vérification des caps. requises
    // Enfin, pour le moment, juste que la table kernel les contient
    verify_capabilities(&manifest)?;

    // chargement des segments natifs
    if flags & FLAG_NATIVE == 0 && flags & FLAG_WASM == 0 {
        return Err(LoadError::UnsupportedFormat);
    }

    let mut entry_vaddr: u64 = 0;
    let mut brk: u64 = 0;

    if flags & FLAG_NATIVE != 0 {
        (entry_vaddr, brk) = load_native_segments(
            pid, data, segments_off, segments_count, entry_offset
        )?;
    }

    if entry_vaddr == 0 { return Err(LoadError::InvalidEntryPoint); }

    // allocation de la pile
    let stack_top = map_stack(pid, USER_STACK_BASE, USER_STACK_SIZE)?;

    let brk_aligned = (brk + 0xFFF) & !0xFFF;

    Ok(LoadedBinary {
        entry: entry_vaddr,
        stack_top,
        brk: brk_aligned,
    })
}

fn parse_header(data: &[u8]) -> Result<&AboHeader, LoadError> {
    if data.len() < HEADER_SIZE { return Err(LoadError::TooSmall); }

    let hdr = unsafe { &*(data.as_ptr() as *const AboHeader) };

    if hdr.magic != ABO_MAGIC { return Err(LoadError::BadMagic); }
    if { hdr.version_major } != ABO_VERSION_MAJ {
        return Err(LoadError::UnsupportedVersion);
    }

    Ok(hdr)
}

fn extract_manifest<'a>(
    data: &'a [u8],
    off: usize,
    size: usize,
) -> Result<Manifest<'a>, LoadError> {
    if size == 0 { return Ok(Manifest::new(b"")); }
    let end = off.checked_add(size).ok_or(LoadError::SegmentOverflow)?;
    if end > data.len() { return Err(LoadError::SegmentOutOfFile); }
    Ok(Manifest::new(&data[off..end]))
}

fn verify_capabilities(manifest: &Manifest) -> Result<(), LoadError> {
    // TODO Phase B : vérifier que le processus appelant a le droit
    for _cap in manifest.cap_required() {
    }
    Ok(())
}

fn load_native_segments(
    pid: ProcessId,
    data: &[u8],
    segments_off: usize,
    count: usize,
    entry_offset: u64,
) -> Result<(u64, u64), LoadError> {
    if count == 0 { return Err(LoadError::BadSegmentTable); }

    let table_end = segments_off
        .checked_add(count * SEGMENT_SIZE)
        .ok_or(LoadError::SegmentOverflow)?;
    if table_end > data.len() { return Err(LoadError::BadSegmentTable); }

    let mut brk: u64 = 0;
    let mut entry_vaddr: u64 = 0;
    let mut first_exec_vaddr: u64 = 0;

    for i in 0..count {
        let seg_off = segments_off + i * SEGMENT_SIZE;
        let aseg = unsafe {
            &*(data[seg_off..].as_ptr() as *const AboSegment)
        };

        let vaddr= { aseg.vaddr };
        let mem_size= { aseg.mem_size } as usize;
        let file_off= { aseg.file_off } as usize;
        let file_size= { aseg.file_size } as usize;
        let seg_flags= { aseg.flags };

        let fend = file_off.checked_add(file_size)
            .ok_or(LoadError::SegmentOverflow)?;
        if fend > data.len() { return Err(LoadError::SegmentOutOfFile); }

        let mut flags = PageTableFlags::PRESENT;
        if seg_flags & SEG_FLAG_W != 0 { flags |= PageTableFlags::WRITABLE; }
        if seg_flags & SEG_FLAG_X == 0 { flags |= PageTableFlags::NO_EXECUTE; }

        let seg = Segment {
            vaddr,
            mem_size,
            file_data: &data[file_off..file_off + file_size],
            flags,
        };

        load_segment(pid, &seg)?;

        if seg_flags & SEG_FLAG_X != 0 && first_exec_vaddr == 0 {
            first_exec_vaddr = vaddr;
        }

        let seg_end = vaddr.saturating_add(mem_size as u64);
        if seg_end > brk { brk = seg_end; }
    }

    if first_exec_vaddr == 0 { return Err(LoadError::InvalidEntryPoint); }

    entry_vaddr = first_exec_vaddr.saturating_add(entry_offset);

    if entry_vaddr >= super::USER_STACK_BASE {
        return Err(LoadError::InvalidEntryPoint);
    }

    Ok((entry_vaddr, brk))
}

pub const ABO_HEADER_SIZE: usize = HEADER_SIZE;

pub fn build_header(
    uuid: [u8; 16],
    flags: u32,
    manifest_off: u32,
    manifest_size: u32,
    segments_off: u32,
    segments_count: u32,
    entry_offset: u32,
) -> [u8; HEADER_SIZE] {
    let mut buf = [0u8; HEADER_SIZE];
    buf[0..4].copy_from_slice(&ABO_MAGIC);
    buf[4..6].copy_from_slice(&ABO_VERSION_MAJ.to_le_bytes());
    buf[6..8].copy_from_slice(&0u16.to_le_bytes());
    buf[8..24].copy_from_slice(&uuid);
    buf[24..28].copy_from_slice(&flags.to_le_bytes());
    buf[28..32].copy_from_slice(&manifest_off.to_le_bytes());
    buf[32..36].copy_from_slice(&manifest_size.to_le_bytes());
    buf[36..40].copy_from_slice(&segments_off.to_le_bytes());
    buf[40..44].copy_from_slice(&segments_count.to_le_bytes());
    buf[44..48].copy_from_slice(&entry_offset.to_le_bytes());
    buf
}

pub fn build_segment_entry(
    vaddr: u64,
    mem_size: u64,
    file_off: u64,
    file_size: u32,
    flags: u32,
) -> [u8; SEGMENT_SIZE] {
    let mut buf = [0u8; SEGMENT_SIZE];
    buf[0..8].copy_from_slice(&vaddr.to_le_bytes());
    buf[8..16].copy_from_slice(&mem_size.to_le_bytes());
    buf[16..24].copy_from_slice(&file_off.to_le_bytes());
    buf[24..28].copy_from_slice(&file_size.to_le_bytes());
    buf[28..32].copy_from_slice(&flags.to_le_bytes());
    buf
}

pub use SEG_FLAG_R as ABO_SEG_R;
pub use SEG_FLAG_W as ABO_SEG_W;
pub use SEG_FLAG_X as ABO_SEG_X;
pub use FLAG_NATIVE as ABO_FLAG_NATIVE;