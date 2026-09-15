//#AbstractConfig:default
//#Object:runtime.c
//#Arch: x86_64

//#Config:error:default
//#ReferenceLinkers:
//#ExpectError:requires executable stack, but -z execstack is not specified

//#Config:allowed:default
//#LinkArgs:-z execstack

//#Config:no-error:default
//#ReferenceLinkers:
//#LinkArgs:--no-error-execstack --warn-execstack
//#ExpectWarningWild:creating an executable stack

//#Config:no-error-silent:default
//#LinkArgs:--no-error-execstack --no-warn-execstack

//#Config:error-flag:default
//#ReferenceLinkers:
//#LinkArgs:--error-execstack
//#ExpectError:requires executable stack, but -z execstack is not specified

.globl _start
_start:
    mov     $42, %rdi
    call    exit_syscall

.section .note.GNU-stack,"x",@progbits
