//! Chargement physique des segments ; partagé ABO et ELF
//! Un segment étant une plage de l'espace virtuel user à remplir avec des données provenant du fichier binaire.

use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};
use crate::process::{self, ProcessId};
use crate::mm::{pmm, vmm, user_ptr::USER_SPACE_END};
use super::LoadError;

pub struct Segment<'a> {
    pub vaddr:u64,
    pub mem_size: usize,
    pub file_data: &'a [u8],
    pub flags: PageTableFlags,
}

pub fn validate_segment(vaddr: u64, mem_size: usize) -> Result<(), LoadError> {
    if mem_size == 0 { return Ok(()); }

    if vaddr == 0 || vaddr >= USER_SPACE_END {
        return Err(LoadError::VaddrInKernelSpace);
    }

    let end = vaddr.checked_add(mem_size as u64)
        .ok_or(LoadError::SegmentOverflow)?;

    if end > USER_SPACE_END {
        return Err(LoadError::VaddrInKernelSpace);
    }

    Ok(())
}

/// Charger un segment dans l'espace d'adressage d'un processus.
///
/// Chargement d'un segment dans l'esp. d'adressage dun processus
/// Pour chaque page couverte par vaddr, vaddr plus mem_size;
/// il faut allouer une frame physique, zéroiser la frame, copier les données du fichier si overlap du file_date puis mapper dans la page table du processus
pub fn load_segment(pid: ProcessId, seg: &Segment) -> Result<(), LoadError> {
    validate_segment(seg.vaddr, seg.mem_size)?;

    if seg.mem_size == 0 { return Ok(()); }

    let page_start = seg.vaddr & !0xFFF;
    let page_end= (seg.vaddr + seg.mem_size as u64 + 0xFFF) & !0xFFF;
    let num_pages= ((page_end - page_start) / 4096) as usize;

    let flags = seg.flags | PageTableFlags::USER_ACCESSIBLE;

    for page_i in 0..num_pages {
        let page_vaddr = page_start + page_i as u64 * 4096;

        let phys = pmm::alloc_frame().ok_or(LoadError::OutOfFrames)?;

        unsafe {
            let frame_virt = vmm::phys_to_virt(phys);
            core::ptr::write_bytes(frame_virt.as_mut_ptr::<u8>(), 0, 4096);

            let page_end_vaddr = page_vaddr + 4096;

            let file_vstart= seg.vaddr;
            let file_vend= seg.vaddr + seg.file_data.len() as u64;

            let copy_vstart= page_vaddr.max(file_vstart);
            let copy_vend= page_end_vaddr.min(file_vend);

            if copy_vstart < copy_vend {
                let copy_len= (copy_vend - copy_vstart) as usize;
                let file_offset= (copy_vstart - file_vstart) as usize;
                let page_offset= (copy_vstart - page_vaddr) as usize;

                let src= seg.file_data[file_offset..file_offset + copy_len].as_ptr();
                let dst= (frame_virt.as_u64() + page_offset as u64) as *mut u8;
                core::ptr::copy_nonoverlapping(src, dst, copy_len);
            }

            process::map_frame(
                pid,
                VirtAddr::new(page_vaddr),
                PhysAddr::new(phys),
                flags,
            ).map_err(|_| LoadError::MappingFailed)?;
        }
    }

    Ok(())
}

pub fn map_stack(pid: ProcessId, stack_base: u64, stack_size: u64) -> Result<u64, LoadError> {
    let num_pages = (stack_size / 4096) as usize;
    let flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::NO_EXECUTE;

    process::map_pages(pid, VirtAddr::new(stack_base), num_pages, flags)
        .map_err(|_| LoadError::StackMappingFailed)?;

    let stack_top = (stack_base + stack_size - 16) & !0xF;
    Ok(stack_top)
}