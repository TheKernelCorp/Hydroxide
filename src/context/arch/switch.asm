.global x86_64_context_switch
.intel_syntax noprefix

# Context Switching
# -----------------
# Context {
#   0x0: flags
#   0x8: rbx
#   0x10: r12
#   0x18: r13
#   0x20: r14
#   0x28: r15
#   0x30: rbp
#   0x38: rsp
# }
#
# rdi <- reference to previous `Context`
# rsi <- reference to next `Context`
x86_64_context_switch:
  pushfq
  pop qword ptr [rdi]

  mov [rdi+0x08], rbx
  mov [rdi+0x10], r12
  mov [rdi+0x18], r13
  mov [rdi+0x20], r14
  mov [rdi+0x28], r15
  mov [rdi+0x30], rbp

  mov [rdi+0x38], rsp
  mov rsp, [rsi+0x38]

  mov rbp, [rsi+0x30]
  mov r15, [rsi+0x28]
  mov r14, [rsi+0x20]
  mov r13, [rsi+0x18]
  mov r12, [rsi+0x10]
  mov rbx, [rsi+0x08]

  push [rsi]
  popfq

  ret