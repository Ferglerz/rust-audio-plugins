_coherent_render:
Lfunc_begin5:
	.cfi_startproc
	cbz	x2, LBB5_19
	stp	d15, d14, [sp, #-160]!
	.cfi_def_cfa_offset 160
	stp	d13, d12, [sp, #16]
	stp	d11, d10, [sp, #32]
	stp	d9, d8, [sp, #48]
	stp	x28, x27, [sp, #64]
	stp	x26, x25, [sp, #80]
	stp	x24, x23, [sp, #96]
	stp	x22, x21, [sp, #112]
	stp	x20, x19, [sp, #128]
	stp	x29, x30, [sp, #144]
	add	x29, sp, #144
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	.cfi_offset w21, -40
	.cfi_offset w22, -48
	.cfi_offset w23, -56
	.cfi_offset w24, -64
	.cfi_offset w25, -72
	.cfi_offset w26, -80
	.cfi_offset w27, -88
	.cfi_offset w28, -96
	.cfi_offset b8, -104
	.cfi_offset b9, -112
	.cfi_offset b10, -120
	.cfi_offset b11, -128
	.cfi_offset b12, -136
	.cfi_offset b13, -144
	.cfi_offset b14, -152
	.cfi_offset b15, -160
	sub	sp, sp, #608
	mov	x20, x1
	mov	x19, x0
	add	x22, x0, #264
	ldrb	w23, [x0, #560]
	lsl	x24, x2, #3
	ldr	d3, [x0, #536]
	ldp	d1, d0, [x0, #392]
	stp	d0, d1, [sp, #48]
	ldp	d1, d0, [x0, #408]
	stp	d0, d1, [sp, #32]
	ldp	d1, d0, [x0, #424]
	stp	d0, d1, [sp, #16]
	ldp	d1, d2, [x0, #440]
	str	d1, [sp, #8]
	ldp	q1, q0, [x22, #192]
	stp	q0, q1, [sp, #80]
	ldr	q0, [x22, #224]
	str	q0, [sp, #64]
	ldr	d0, [x0, #520]
	stp	d0, d3, [sp, #128]
	ldr	d0, [x0, #528]
	stp	d2, d0, [sp, #112]
	ldr	d1, [x0, #224]
	ldr	d0, [x0, #112]
	stp	d0, d1, [sp, #384]
	ldr	d15, [x0, #168]
	ldr	d0, [x0, #280]
	str	d0, [sp, #376]
	ldur	q1, [x0, #120]
	ldur	q0, [x0, #232]
	stp	q0, q1, [sp, #336]
	ldur	q1, [x0, #136]
	ldp	q2, q3, [x0, #176]
	stp	q2, q1, [sp, #304]
	ldur	q0, [x0, #248]
	stp	q0, q3, [sp, #272]
	ldp	q1, q0, [x0, #288]
	stp	q0, q1, [sp, #240]
	ldur	q1, [x0, #152]
	ldr	q0, [x0, #208]
	stp	q0, q1, [sp, #208]
	ldr	q1, [x22]
	ldr	q0, [x0, #320]
	stp	q0, q1, [sp, #176]
	ldr	d11, [x0, #552]
	ldr	d12, [x0, #544]
	ldp	x25, x26, [x0, #504]
	ldrb	w27, [x0, #561]
	mov	x28, #48234
	movk	x28, #37748, lsl #16
	movk	x28, #1048, lsl #32
	movk	x28, #16368, lsl #48
Lloh128:
	adrp	x21, l_.memset_pattern@PAGE
Lloh129:
	add	x21, x21, l_.memset_pattern@PAGEOFF
	ldr	d13, [x0, #336]
	ldp	q1, q3, [x22, #80]
	ldr	q5, [x22, #112]
	ldr	d10, [x0]
	ldr	d8, [x0, #56]
	ldur	q20, [x0, #8]
	ldur	q21, [x0, #24]
	ldp	q2, q0, [x0, #64]
	stp	q2, q0, [x29, #-208]
	fmov	d16, #1.00000000
	ldur	q6, [x0, #40]
	ldr	q7, [x0, #96]
	b	LBB5_3
LBB5_2:
	ldr	q0, [sp, #336]
	fmul.2d	v0, v0, v19
	fmul.2d	v0, v0, v22[0]
	ldr	q4, [sp, #400]
	fmul.2d	v0, v4, v0
	mov	d1, v0[1]
	ldr	d2, [sp, #392]
	fmul	d2, d2, d14
	fmul	d2, d2, d22
	fmul	d2, d9, d2
	movi.2d	v3, #0000000000000000
	fadd	d2, d2, d3
	fadd	d0, d2, d0
	fadd	d0, d1, d0
	ldr	q1, [sp, #272]
	fmul.2d	v1, v1, v18
	fmul.2d	v1, v1, v22[0]
	ldr	q3, [sp, #480]
	fmul.2d	v1, v3, v1
	mov	d2, v1[1]
	fadd	d0, d1, d0
	fadd	d0, d2, d0
	ldr	q1, [sp, #192]
	fmul.2d	v1, v1, v17
	fmul.2d	v1, v1, v22[0]
	fmul.2d	v1, v5, v1
	mov	d2, v1[1]
	fadd	d0, d1, d0
	fadd	d0, d2, d0
	ldr	d1, [x20]
	fadd	d0, d0, d1
	str	d0, [x20], #8
	ldp	q0, q2, [sp, #240]
	fmul.2d	v1, v2, v4
	fmul.2d	v3, v0, v3
	ldr	q0, [sp, #176]
	fmul.2d	v5, v0, v5
	add	x25, x25, #1
	subs	x24, x24, #8
	b.eq	LBB5_18
LBB5_3:
	mov.16b	v14, v10
	mov.16b	v19, v20
	mov.16b	v18, v21
	mov.16b	v17, v6
	stp	q21, q6, [sp, #432]
	str	q20, [sp, #416]
	tbz	w23, #0, LBB5_8
	fadd	d12, d12, d16
	str	d12, [x19, #544]
	ldr	d10, [sp, #136]
	tbnz	w27, #0, LBB5_7
	fcmp	d12, d10
	b.le	LBB5_12
	mov	w8, #1
	strb	w8, [x19, #561]
LBB5_7:
	ldr	d0, [sp, #112]
	fmul	d9, d0, d13
	ldr	q0, [sp, #96]
	fmul.2d	v0, v0, v1
	str	q0, [sp, #400]
	ldp	q0, q1, [sp, #64]
	fmul.2d	v3, v1, v3
	str	q3, [sp, #480]
	mov	w27, #1
	fmul.2d	v5, v0, v5
	fmov	d22, #1.00000000
	cmp	x25, x26
	stur	q5, [x29, #-256]
	b.lo	LBB5_9
	b	LBB5_15
LBB5_8:
	str	q3, [sp, #480]
	str	q1, [sp, #400]
	mov.16b	v9, v13
	fmov	d22, #1.00000000
	cmp	x25, x26
	stur	q5, [x29, #-256]
	b.hs	LBB5_15
LBB5_9:
	str	q7, [sp, #464]
	ucvtf	d0, x25
	ldr	d1, [sp, #128]
	fmul	d0, d1, d0
	bl	_cos
	fmov	d16, #1.00000000
	fsub	d0, d16, d0
	fmov	d1, #0.50000000
	fmul	d22, d0, d1
	fmov	d0, x28
	ldr	d1, [sp, #120]
	fcmp	d1, d0
	b.ls	LBB5_14
	mov	x8, #17302
	movk	x8, #27787, lsl #16
	movk	x8, #64487, lsl #32
	movk	x8, #16383, lsl #48
	fmov	d0, x8
	fcmp	d1, d0
	b.ge	LBB5_13
	mov.16b	v0, v22
	bl	_pow
	fmov	d16, #1.00000000
	mov.16b	v22, v0
	b	LBB5_14
LBB5_12:
	ldr	d0, [sp, #56]
	fnmul	d0, d12, d0
	fdiv	d0, d0, d10
	stur	q5, [x29, #-256]
	stp	q7, q3, [sp, #464]
	stur	q1, [x29, #-224]
	bl	_exp
	fmul	d9, d13, d0
	ldr	d0, [sp, #48]
	fnmul	d0, d12, d0
	fdiv	d0, d0, d10
	bl	_exp
	stur	q0, [x29, #-240]
	ldr	d0, [sp, #40]
	fnmul	d0, d12, d0
	fdiv	d0, d0, d10
	bl	_exp
	ldur	q1, [x29, #-240]
	mov.d	v1[1], v0[0]
	ldur	q0, [x29, #-224]
	fmul.2d	v0, v0, v1
	str	q0, [sp, #400]
	ldr	d0, [sp, #32]
	fnmul	d0, d12, d0
	fdiv	d0, d0, d10
	bl	_exp
	stur	q0, [x29, #-224]
	ldr	d0, [sp, #24]
	fnmul	d0, d12, d0
	fdiv	d0, d0, d10
	bl	_exp
	ldur	q1, [x29, #-224]
	mov.d	v1[1], v0[0]
	ldr	q0, [sp, #480]
	fmul.2d	v0, v0, v1
	str	q0, [sp, #480]
	ldr	d0, [sp, #16]
	fnmul	d0, d12, d0
	fdiv	d0, d0, d10
	bl	_exp
	stur	q0, [x29, #-224]
	ldr	d0, [sp, #8]
	fnmul	d0, d12, d0
	fdiv	d0, d0, d10
	bl	_exp
	ldp	q19, q18, [sp, #416]
	ldp	q17, q7, [sp, #448]
	fmov	d16, #1.00000000
	ldur	q5, [x29, #-256]
	mov	w27, #0
	ldur	q1, [x29, #-224]
	mov.d	v1[1], v0[0]
	fmul.2d	v5, v5, v1
	fmov	d22, #1.00000000
	cmp	x25, x26
	stur	q5, [x29, #-256]
	b.lo	LBB5_9
	b	LBB5_15
LBB5_13:
	fmul	d22, d22, d22
LBB5_14:
	ldur	q5, [x29, #-256]
	ldp	q17, q7, [sp, #448]
	ldp	q19, q18, [sp, #416]
LBB5_15:
	ldr	d2, [sp, #384]
	fmul	d0, d2, d14
	fmul	d1, d15, d8
	fadd	d10, d1, d0
	fmul	d0, d2, d8
	fmul	d1, d15, d14
	fsub	d8, d0, d1
	ldr	d0, [sp, #376]
	fmul	d0, d0, d9
	ldr	q3, [sp, #352]
	fmul.2d	v1, v3, v19
	ldr	q4, [sp, #304]
	ldur	q6, [x29, #-208]
	fmul.2d	v2, v4, v6
	fadd.2d	v20, v2, v1
	fmul.2d	v1, v3, v6
	fmul.2d	v2, v4, v19
	fsub.2d	v1, v1, v2
	stur	q1, [x29, #-208]
	ldr	q3, [sp, #320]
	fmul.2d	v1, v3, v18
	ldr	q4, [sp, #288]
	ldur	q6, [x29, #-192]
	fmul.2d	v2, v4, v6
	fadd.2d	v21, v2, v1
	fmul.2d	v1, v3, v6
	fmul.2d	v2, v4, v18
	fsub.2d	v1, v1, v2
	stur	q1, [x29, #-192]
	ldp	q4, q3, [sp, #208]
	fmul.2d	v1, v3, v17
	fmul.2d	v2, v4, v7
	fadd.2d	v6, v2, v1
	fmul.2d	v1, v3, v7
	fmul.2d	v2, v4, v17
	fsub.2d	v7, v1, v2
	fmul	d1, d11, d0
	fsub	d1, d16, d1
	fmul	d1, d0, d1
	fcmp	d11, #0.0
	fcsel	d13, d1, d0, gt
	cbz	x25, LBB5_2
	and	x8, x25, #0x3ff
	cbnz	x8, LBB5_2
	fmul	d0, d10, d10
	fmul	d1, d8, d8
	fadd	d0, d0, d1
	fsqrt	d0, d0
	fdiv	d0, d16, d0
	fmul	d10, d10, d0
	fmul	d8, d8, d0
	sub	x0, x29, #176
	mov	x1, x21
	mov	w2, #16
	str	q7, [sp, #464]
	stp	q21, q20, [x29, #-240]
	stp	q22, q6, [sp, #144]
	bl	_memset_pattern16
	ldp	q0, q2, [x29, #-224]
	fmul.2d	v0, v0, v0
	fmul.2d	v1, v2, v2
	fadd.2d	v0, v0, v1
	ldur	q1, [x29, #-176]
	fsqrt.2d	v0, v0
	fdiv.2d	v0, v1, v0
	ldur	q1, [x29, #-224]
	fmul.2d	v1, v1, v0
	fmul.2d	v2, v2, v0
	stp	q1, q2, [x29, #-224]
	sub	x0, x29, #176
	mov	x1, x21
	mov	w2, #16
	bl	_memset_pattern16
	ldur	q0, [x29, #-240]
	fmul.2d	v0, v0, v0
	ldur	q2, [x29, #-192]
	fmul.2d	v1, v2, v2
	fadd.2d	v0, v0, v1
	ldur	q1, [x29, #-176]
	fsqrt.2d	v0, v0
	fdiv.2d	v0, v1, v0
	ldur	q1, [x29, #-240]
	fmul.2d	v1, v1, v0
	stur	q1, [x29, #-240]
	fmul.2d	v2, v2, v0
	stur	q2, [x29, #-192]
	sub	x0, x29, #176
	mov	x1, x21
	mov	w2, #16
	bl	_memset_pattern16
	ldp	q22, q6, [sp, #144]
	ldp	q21, q20, [x29, #-240]
	ldp	q19, q18, [sp, #416]
	ldp	q17, q7, [sp, #448]
	fmov	d16, #1.00000000
	ldur	q5, [x29, #-256]
	fmul.2d	v0, v6, v6
	fmul.2d	v1, v7, v7
	fadd.2d	v0, v0, v1
	ldur	q1, [x29, #-176]
	fsqrt.2d	v0, v0
	fdiv.2d	v0, v1, v0
	fmul.2d	v6, v6, v0
	fmul.2d	v7, v7, v0
	b	LBB5_2
LBB5_18:
	str	d13, [x19, #336]
	stp	q1, q3, [x22, #80]
	str	q5, [x22, #112]
	str	x25, [x19, #504]
	str	d10, [x19]
	str	d8, [x19, #56]
	stur	q20, [x19, #8]
	stur	q21, [x19, #24]
	ldp	q1, q0, [x29, #-208]
	stp	q1, q0, [x19, #64]
	stur	q6, [x19, #40]
	str	q7, [x19, #96]
	add	sp, sp, #608
	.cfi_def_cfa wsp, 160
	ldp	x29, x30, [sp, #144]
	ldp	x20, x19, [sp, #128]
	ldp	x22, x21, [sp, #112]
	ldp	x24, x23, [sp, #96]
	ldp	x26, x25, [sp, #80]
	ldp	x28, x27, [sp, #64]
	ldp	d9, d8, [sp, #48]
	ldp	d11, d10, [sp, #32]
	ldp	d13, d12, [sp, #16]
	ldp	d15, d14, [sp], #160
	.cfi_def_cfa_offset 0
	.cfi_restore w30
	.cfi_restore w29
	.cfi_restore w19
	.cfi_restore w20
	.cfi_restore w21
	.cfi_restore w22
	.cfi_restore w23
	.cfi_restore w24
	.cfi_restore w25
	.cfi_restore w26
	.cfi_restore w27
	.cfi_restore w28
	.cfi_restore b8
	.cfi_restore b9
	.cfi_restore b10
	.cfi_restore b11
	.cfi_restore b12
	.cfi_restore b13
	.cfi_restore b14
	.cfi_restore b15
LBB5_19:
	ret
	.loh AdrpAdd	Lloh128, Lloh129
Lfunc_end5:
	