use x86_64::structures::paging::PageTableFlags;
use crate::process::ProcessId;
use super::{LoadError, LoadedBinary, USER_STACK_BASE, USER_STACK_SIZE};
use super::segments::{Segment, load_segment, map_stack};

const ELF_MAGIC:[u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ET_EXEC: u16 = 2;
const ET_DYN:u16 = 3;   // PIE
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 0x1;
const PF_W: u32 = 0x2;

#[repr(C, packed)]
struct Elf64Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff:u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C, packed)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

pub fn load(pid: ProcessId, data: &[u8]) -> Result<LoadedBinary, LoadError> {
    let ehdr = parse_header(data)?;

    let e_type= { ehdr.e_type };
    let e_entry = { ehdr.e_entry };
    let e_phoff = { ehdr.e_phoff } as usize;
    let e_phnum = { ehdr.e_phnum } as usize;
    let e_phentsize= { ehdr.e_phentsize } as usize;

    if e_phentsize < core::mem::size_of::<Elf64Phdr>() {
        return Err(LoadError::BadSegmentTable);
    }

    let load_bias: u64 = if e_type == ET_DYN {
        super::USER_CODE_BASE
    } else {
        0
    };

    let mut brk: u64 = 0;

    for i in 0..e_phnum {
        let phdr_off= e_phoff + i * e_phentsize;
        let phdr= read_phdr(data, phdr_off)?;

        let p_type= { phdr.p_type };
        let p_flags= { phdr.p_flags };
        let p_offset= { phdr.p_offset } as usize;
        let p_vaddr= { phdr.p_vaddr };
        let p_filesz = { phdr.p_filesz } as usize;
        let p_memsz= { phdr.p_memsz } as usize;

        if p_type != PT_LOAD { continue; }
        if p_memsz == 0 { continue; }

        let vaddr = p_vaddr.wrapping_add(load_bias);

        let file_end = p_offset.checked_add(p_filesz)
            .ok_or(LoadError::SegmentOverflow)?;
        if file_end > data.len() {
            return Err(LoadError::SegmentOutOfFile);
        }

        let mut flags = PageTableFlags::PRESENT;
        if p_flags & PF_W != 0 { flags |= PageTableFlags::WRITABLE; }
        if p_flags & PF_X == 0 { flags |= PageTableFlags::NO_EXECUTE; }

        let seg = Segment {
            vaddr,
            mem_size:  p_memsz,
            file_data: &data[p_offset..p_offset + p_filesz],
            flags,
        };

        load_segment(pid, &seg)?;

        let seg_end = vaddr.saturating_add(p_memsz as u64);
        if seg_end > brk { brk = seg_end; }
    }

    if brk == 0 { return Err(LoadError::BadSegmentTable); }

    let entry = e_entry.wrapping_add(load_bias);
    validate_entry(entry, brk)?;

    let stack_top = map_stack(pid, USER_STACK_BASE, USER_STACK_SIZE)?;

    let brk_aligned = (brk + 0xFFF) & !0xFFF;

    Ok(LoadedBinary { entry, stack_top, brk: brk_aligned })
}

fn parse_header(data: &[u8]) -> Result<&Elf64Ehdr, LoadError> {
    if data.len() < core::mem::size_of::<Elf64Ehdr>() {
        return Err(LoadError::TooSmall);
    }

    let ehdr = unsafe { &*(data.as_ptr() as *const Elf64Ehdr) };

    if &ehdr.e_ident[..4] != &ELF_MAGIC { return Err(LoadError::BadMagic); }
    if ehdr.e_ident[4] != ELFCLASS64 { return Err(LoadError::UnsupportedArch); }
    if ehdr.e_ident[5] != ELFDATA2LSB { return Err(LoadError::UnsupportedFormat); }

    let e_type= { ehdr.e_type };
    let e_machine= { ehdr.e_machine };

    if e_type != ET_EXEC && e_type != ET_DYN { return Err(LoadError::UnsupportedFormat); }
    if e_machine != EM_X86_64 { return Err(LoadError::UnsupportedArch); }

    Ok(ehdr)
}

fn read_phdr(data: &[u8], offset: usize) -> Result<&Elf64Phdr, LoadError> {
    let end = offset.checked_add(core::mem::size_of::<Elf64Phdr>())
        .ok_or(LoadError::BadSegmentTable)?;
    if end > data.len() { return Err(LoadError::BadSegmentTable); }
    Ok(unsafe { &*(data[offset..].as_ptr() as *const Elf64Phdr) })
}

fn validate_entry(entry: u64, brk: u64) -> Result<(), LoadError> {
    if entry == 0 || entry >= super::USER_STACK_BASE {
        return Err(LoadError::InvalidEntryPoint);
    }
    Ok(())
}