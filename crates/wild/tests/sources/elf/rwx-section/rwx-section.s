// Test that sections with writable+executable (awx) flags are placed in an RWX LOAD segment.
// Wild should warn about RWX permissions like GNU ld does.
//#AbstractConfig:default
//#Arch:x86_64
//#Mode:static
//#RunEnabled:false

//#Config:warn:default
//#ExpectWarningWild:has RWX \(read\+write\+execute\) permissions

//#Config:no-warn:default
//#ReferenceLinkers:
//#LinkArgs:--no-warn-rwx-segments --fatal-warnings

//#Config:fatal:default
//#ReferenceLinkers:
//#LinkArgs:--warn-rwx-segments --fatal-warnings
//#ExpectErrorWild:warnings being treated as errors

.section .wtext,"awx"
.globl _start
_start:
    ret
