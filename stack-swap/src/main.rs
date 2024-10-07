use core::arch::asm;
#[derive(Debug, Default)]
#[repr(C)]
struct ThreadContext {
    rsp: u64,
}

const SSIZE: usize = 8192;

fn hello() -> ! {
    println!("Hello called");
    loop {}
}

unsafe fn gt_switch(new: *const ThreadContext) {
    asm!(
        "mov rsp, [{0} + 0x00]",
        "ret",
        in(reg) new,
        );
}

fn main() {
    let mut ctx = ThreadContext::default();
    let mut stack = vec![0_u8; SSIZE];

    unsafe {
        let stack_bottom = stack.as_mut_ptr().offset(SSIZE.try_into().unwrap());
        let sb_aligned = (stack_bottom as usize & !15) as *mut u8;
        std::ptr::write(sb_aligned.offset(-16) as *mut u64, hello as u64);
        ctx.rsp = sb_aligned.offset(-16) as u64;
        gt_switch(&mut ctx);
    }
}
