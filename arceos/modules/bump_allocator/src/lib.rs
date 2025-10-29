#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};
use core::ptr::NonNull;
use allocator::AllocError; 
/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
	start: usize,      // 内存起始地址
    end: usize,        // 内存结束地址  
    b_pos: usize,      // 字节分配当前位置
    p_pos: usize,      // 页分配当前位置
    count: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
			start:0,
			end:0,
			b_pos:0,
			p_pos:0,
			count:0
		}
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start=start;
		self.end=start+size;
		self.b_pos=self.start;
		self.p_pos=self.end;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> allocator::AllocResult {
        todo!()
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(
        &mut self,
        layout: core::alloc::Layout,
    ) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
		let need=layout.pad_to_align().size();
		if (self.b_pos+need)>self.p_pos{
			return Err(AllocError::MemoryOverlap);
		}
		let ret=self.b_pos;
		self.b_pos+=need;
		self.count+=1;
		Ok(NonNull::new(ret as *mut u8).unwrap())
    }

    fn dealloc(&mut self, pos: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        if self.count > 0 {
			self.count -= 1;
			if self.count == 0 {
				self.b_pos = self.start;
			}
		}
    }

    fn total_bytes(&self) -> usize {
        self.end-self.start
    }

    fn used_bytes(&self) -> usize {
        self.b_pos-self.start
    }

    fn available_bytes(&self) -> usize {
        self.p_pos-self.b_pos
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(
        &mut self,
        num_pages: usize,
        align_pow2: usize,
    ) -> allocator::AllocResult<usize> {
        if !align_pow2.is_power_of_two() || align_pow2 < PAGE_SIZE {
			return Err(AllocError::InvalidParam);
		}
	
		let total_bytes = num_pages * PAGE_SIZE;
	
		// 确保有足够空间
		if total_bytes > self.p_pos || self.p_pos - total_bytes < self.b_pos {
			return Err(AllocError::NoMemory);
		}
	
		// 从高地址向低地址分配，并对齐
		let unaligned = self.p_pos - total_bytes;
		let aligned = unaligned & !(align_pow2 - 1); // 向下对齐
	
		if aligned < self.b_pos {
			return Err(AllocError::NoMemory);
		}
	
		self.p_pos = aligned;
		Ok(aligned)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        
    }

    fn total_pages(&self) -> usize {
        (self.end-self.start)/PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.end-self.p_pos)/PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        (self.p_pos-self.b_pos)/PAGE_SIZE
    }
}