use std::env;
use std::mem;
use std::ptr;
use std::process::ExitCode;
use iced_x86::{Decoder, DecoderOptions, Instruction, Code};
use std::sync::atomic::{AtomicBool, Ordering};


// Индексы регистров в массиве ucontext_t.gregs на Linux x86_64
// (значения из sys/ucontext.h)
const REG_RDX: usize = 12;
const REG_RAX: usize = 13;
const REG_RIP: usize = 16;

// Обработчик сигнала был вызван.
static SIGFPE_HANDLED: AtomicBool = AtomicBool::new(false);


unsafe extern "C" fn sigfpe_handler(
    _sig: libc::c_int,
    info: *mut libc::siginfo_t,
    context: *mut libc::c_void
) {
    let code = unsafe { (*info).si_code };
    if code != 1 {
        // Не целочисленное деление на ноль — восстанавливаем поведение
        // по умолчанию (обычно завершение процесса).
        unsafe { libc::abort() };
    }

    let ctx = unsafe { &mut *(context as *mut libc::ucontext_t) };

    let rip = ctx.uc_mcontext.gregs[REG_RIP as usize] as *const u8;
    if rip.is_null() {
        unsafe { libc::abort() };
    }

    //  Декодируем инструкцию по адресу RIP с помощью iced-x86.
    //  Максимальная длина инструкции x86 — 15 байт.
    let code_bytes = unsafe { std::slice::from_raw_parts(rip, 15) };

    let mut decoder = Decoder::with_ip(64, code_bytes, rip as u64, DecoderOptions::NONE);
    let mut instruction = Instruction::default();
    decoder.decode_out(&mut instruction);

    if instruction.is_invalid() {
        unsafe { libc::abort() };
    }

    // Проверяем все варианты, что это действительно div или idiv
    let is_div_or_idiv = matches!(
        instruction.code(),
        Code::Div_rm8
            | Code::Div_rm16
            | Code::Div_rm32
            | Code::Div_rm64
            | Code::Idiv_rm8
            | Code::Idiv_rm16
            | Code::Idiv_rm32
            | Code::Idiv_rm64
    );
    
    if !is_div_or_idiv {
        // Это не div/idiv — не наш случай, восстанавливаем поведение
        // по умолчанию
        unsafe { libc::abort() };
    }

    // Инструкция распознана: обнуляем RAX (частное) и RDX (остаток)
    ctx.uc_mcontext.gregs[REG_RAX as usize] = 0;
    ctx.uc_mcontext.gregs[REG_RDX as usize] = 0;

    // Продвигаем RIP на длину инструкции, чтобы CPU перешагнул
    // сбойный div/idiv при возврате из обработчика
    let len = instruction.len() as i64;
    ctx.uc_mcontext.gregs[REG_RIP as usize] += len;

    // Устанавливаем флаг
    SIGFPE_HANDLED.store(true, Ordering::SeqCst);
}


fn install_sigfpe_handler() {
    unsafe {
        let mut action: libc::sigaction = mem::zeroed();
        action.sa_sigaction = sigfpe_handler as usize;
        action.sa_flags = libc::SA_SIGINFO; // Обязательно! Иначе ucontext не придёт
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGFPE, &action, ptr::null_mut());
    }
}


fn main() -> ExitCode {
    install_sigfpe_handler();


    let mut args = env::args().skip(1);
    
    let a: u64 = match args.next() {
        Some(arg) => if let Ok(p) = arg.parse::<u64>() { p } else { 0 as u64 },
        None => {
            println!("Numeric argument is needed!");
            return ExitCode::FAILURE;
        }
    };
    
    let b: u64 = match args.next() {
        Some(arg) => if let Ok(p) = arg.parse::<u64>() { p } else { 0 as u64 },
        None => {
            println!("Numeric argument is needed!");
            return ExitCode::FAILURE;
        }        
    };
    
    let mut quotient: u64;
    let mut remainder: u64;

    // Готовим деление, которое вызовет деление на ноль.
    // Для 64-битного div/idiv делимое лежит в RDX:RAX.
    // Используем inline asm, чтобы гарантированно выполнить div.    
    unsafe {
        std::arch::asm!(
            "xor rdx, rdx",       // RDX = 0 (старшие 64 бита делимого)
            "mov rax, {a}",       // RAX = a
            "div {b}",            // Вероятное (зависит от аргументов) деление на ноль → #DE → SIGFPE
            a = in(reg) a,
            b = in(reg) b,
            out("rax") quotient,
            out("rdx") remainder,
        );
    }

    if SIGFPE_HANDLED.load(Ordering::SeqCst) {
        // Если обработчик сработал успешно, RAX и RDX обнулены,
        // RIP перепрыгнул div, и мы оказались здесь
        println!("Восстановились после SIGFPE: quotient = {}, remainder = {}", quotient, remainder);
    } else {
        // Деление завершено успешно
        println!("quotien = {}, reminder = {}", quotient, remainder);
    }

    ExitCode::SUCCESS
}
