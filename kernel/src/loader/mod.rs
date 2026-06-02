use x86_64::{
    structures::paging::PageTableFlags,
    VirtAddr,
};

use crate::process::{self, ProcessId};
use crate::mm::{pmm, vmm};

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1; // little-endian
const ET_EXEC: u16 = 2; // exécutable
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;

const PF_X: u32 = 0x1; // execute
const PF_W: u32 = 0x2; // write
const PF_R: u32 = 0x4; // read

#[repr(C, packed)]
struct Elf64Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum:u16,
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

#[derive(Debug)]
pub enum ElfError {
    TooSmall,
    BadMagic,
    NotElf64,
    NotLittleEndian,
    NotExecutable,
    NotX86_64,
    BadProgramHeader,
    VaddrTooHigh,
    PmmOutOfFrames,
    MappingFailed,
    SegmentOutOfFile,
}

pub struct LoadedElf {
    pub entry:     u64,
    pub stack_top: u64,
}

pub fn load_elf(pid: ProcessId, elf_data: &[u8]) -> Result<LoadedElf, ElfError> {
    if elf_data.len() < core::mem::size_of::<Elf64Header>() {
        return Err(ElfError::TooSmall);
    }

    let header = unsafe {
        &*(elf_data.as_ptr() as *const Elf64Header)
    };

    if &header.e_ident[..4] != &ELF_MAGIC {
        return Err(ElfError::BadMagic);
    }
    if header.e_ident[4] != ELFCLASS64 {
        return Err(ElfError::NotElf64);
    }
    if header.e_ident[5] != ELFDATA2LSB {
        return Err(ElfError::NotLittleEndian);
    }

    let e_type = { header.e_type };
    let e_machine= { header.e_machine };
    let e_entry= { header.e_entry };
    let e_phoff = { header.e_phoff };
    let e_phnum= { header.e_phnum };
    let e_phentsize= { header.e_phentsize };

    if e_type != ET_EXEC {
        return Err(ElfError::NotExecutable);
    }
    if e_machine != EM_X86_64 {
        return Err(ElfError::NotX86_64);
    }

    let mut highest_vaddr: u64 = 0;

    for i in 0..e_phnum as usize {
        let phdr_offset = e_phoff as usize + i * e_phentsize as usize;
        if phdr_offset + core::mem::size_of::<Elf64Phdr>() > elf_data.len() {
            return Err(ElfError::BadProgramHeader);
        }

        let phdr = unsafe {
            &*(elf_data[phdr_offset..].as_ptr() as *const Elf64Phdr)
        };

        let p_type= { phdr.p_type };
        let p_flags= { phdr.p_flags };
        let p_offset= { phdr.p_offset };
        let p_vaddr= { phdr.p_vaddr };
        let p_filesz= { phdr.p_filesz };
        let p_memsz= { phdr.p_memsz };

        if p_type != PT_LOAD { continue; }

        if p_vaddr >= crate::mm::user_ptr::USER_SPACE_END {
            return Err(ElfError::VaddrTooHigh);
        }

        let src_end = p_offset.checked_add(p_filesz)
            .ok_or(ElfError::SegmentOutOfFile)? as usize;
        if src_end > elf_data.len() {
            return Err(ElfError::SegmentOutOfFile);
        }

        let page_start= p_vaddr & !0xFFF;
        let page_end= (p_vaddr + p_memsz + 0xFFF) & !0xFFF;
        let num_pages = ((page_end - page_start) / 4096) as usize;

        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        if p_flags & PF_W != 0 {
            flags |= PageTableFlags::WRITABLE;
        }
        if p_flags & PF_X == 0 {
            flags |= PageTableFlags::NO_EXECUTE;
        }

        unsafe {
            load_segment(
                pid, page_start, num_pages, flags,
                elf_data, p_offset as usize, p_filesz as usize,
                p_vaddr, p_memsz as usize,
            )?;
        }

        let seg_end = p_vaddr + p_memsz;
        if seg_end > highest_vaddr { highest_vaddr = seg_end; }
    }

    let stack_base = (highest_vaddr + 0xFFFF) & !0xFFFF;
    let stack_size = 64 * 1024u64; // 64 KB
    let stack_pages = (stack_size / 4096) as usize;
    let stack_flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::NO_EXECUTE;

    process::map_pages(pid, VirtAddr::new(stack_base), stack_pages, stack_flags)
        .map_err(|_| ElfError::MappingFailed)?;

    let stack_top = stack_base + stack_size - 16;

    Ok(LoadedElf { entry: e_entry, stack_top })
}

unsafe fn load_segment(
    pid: ProcessId,
    page_start: u64,
    num_pages: usize,
    flags: PageTableFlags,
    elf_data: &[u8],
    file_offset: usize,
    file_size: usize,
    vaddr: u64,
    mem_size: usize,
) -> Result<(), ElfError> {
    use crate::mm::pmm;

    for page_i in 0..num_pages {
        let page_vaddr = page_start + page_i as u64 * 4096;

        let phys = pmm::alloc_frame().ok_or(ElfError::PmmOutOfFrames)?;

        let frame_virt = vmm::phys_to_virt(phys);
        core::ptr::write_bytes(frame_virt.as_mut_ptr::<u8>(), 0, 4096);

        process::map_frame(pid, VirtAddr::new(page_vaddr), x86_64::PhysAddr::new(phys), flags)
            .map_err(|_| ElfError::MappingFailed)?;

        let page_end= page_vaddr + 4096;
        let data_start= vaddr;
        let data_end= vaddr + file_size as u64;

        let copy_start= page_vaddr.max(data_start);
        let copy_end= page_end.min(data_end);

        if copy_start < copy_end {
            let copy_len= (copy_end - copy_start) as usize;
            let file_src = file_offset + (copy_start - data_start) as usize;
            let page_dst= (copy_start - page_vaddr) as usize;

            let dst = (frame_virt.as_u64() + page_dst as u64) as *mut u8;
            let src = elf_data[file_src..file_src + copy_len].as_ptr();
            core::ptr::copy_nonoverlapping(src, dst, copy_len);
        }
    }

    Ok(())
}