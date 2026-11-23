_simd_render:
Lfunc_begin7:
	.cfi_startproc
	cbz	x2, LBB7_21
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
	sub	sp, sp, #512
	mov	x20, x1
	mov	x19, x0
	add	x8, x0, #280
	ldrb	w21, [x0, #560]
	lsl	x22, x2, #3
	ldr	d0, [x0, #536]
	str	d0, [sp, #88]
	ldp	d1, d0, [x0, #400]
	stp	d0, d1, [sp, #48]
	ldp	d1, d0, [x0, #416]
	stp	d0, d1, [sp, #32]
	ldp	d1, d0, [x0, #432]
	stp	d0, d1, [sp, #16]
	ldp	q1, q0, [x0, #448]
	stp	q0, q1, [sp, #112]
	ldr	q0, [x0, #480]
	str	q0, [sp, #96]
	ldr	d15, [x0, #496]
	ldr	d1, [x0, #520]
	ldr	d0, [x0, #528]
	stp	d0, d1, [sp, #144]
	ldur	q1, [x0, #168]
	ldp	q2, q3, [x0, #112]
	stp	q2, q1, [sp, #336]
	ldur	q0, [x0, #184]
	stp	q0, q3, [sp, #304]
	ldp	q1, q0, [x0, #224]
	stp	q0, q1, [sp, #272]
	ldp	q1, q0, [x8]
	stp	q0, q1, [sp, #240]
	ldr	q1, [x0, #144]
	ldur	q0, [x0, #200]
	stp	q0, q1, [sp, #208]
	ldr	q1, [x0, #256]
	ldr	q0, [x8, #32]
	stp	q0, q1, [sp, #176]
	ldr	d1, [x0, #272]
	ldr	d13, [x0, #160]
	ldr	d14, [x0, #216]
	ldr	d0, [x0, #328]
	stp	d0, d1, [sp, #160]
	ldr	d10, [x0, #552]
	ldr	d11, [x0, #544]
	ldp	x23, x24, [x0, #504]
	ldrb	w28, [x0, #561]
	mov	w25, #1
	mov	x26, #48234
	movk	x26, #37748, lsl #16
	movk	x26, #1048, lsl #32
	movk	x26, #16368, lsl #48
	mov	x27, #17302
	movk	x27, #27787, lsl #16
	movk	x27, #64487, lsl #32
	movk	x27, #16383, lsl #48
	ldp	d8, d0, [x0, #384]
	stp	d0, d15, [sp]
	ldr	d4, [x0, #48]
	ldr	d12, [x0, #104]
	ldp	q17, q16, [x0, #352]
	ldur	q19, [x0, #56]
	ldp	q1, q2, [x0]
	ldur	q18, [x0, #72]
	fmov	d21, #1.00000000
	ldr	q3, [x0, #32]
	ldur	q20, [x0, #88]
	b	LBB7_3
LBB7_2:
	ldr	q6, [sp, #288]
	fmul.2d	v6, v6, v25
	fmul.2d	v6, v6, v0[0]
	fmul.2d	v5, v5, v6
	mov	d6, v5[1]
	movi.2d	v7, #0000000000000000
	fadd	d5, d5, d7
	fadd	d5, d6, d5
	ldr	q6, [sp, #272]
	fmul.2d	v6, v6, v24
	fmul.2d	v6, v6, v0[0]
	fmul.2d	v6, v17, v6
	mov	d7, v6[1]
	fadd	d5, d5, d6
	fadd	d5, d7, d5
	ldr	q6, [sp, #192]
	fmul.2d	v6, v6, v23
	fmul.2d	v6, v6, v0[0]
	fmul.2d	v6, v16, v6
	mov	d7, v6[1]
	fadd	d5, d5, d6
	fadd	d5, d7, d5
	ldr	d6, [sp, #168]
	fmul	d6, d6, d9
	fmul	d0, d6, d0
	fmul	d0, d8, d0
	fadd	d0, d0, d5
	ldr	d5, [x20]
	fadd	d0, d0, d5
	str	d0, [x20], #8
	ldr	q0, [sp, #240]
	fmul.2d	v17, v0, v17
	ldr	q0, [sp, #176]
	fmul.2d	v16, v0, v16
	ldr	d0, [sp, #160]
	fmul	d8, d0, d8
	add	x23, x23, #1
	subs	x22, x22, #8
	b.eq	LBB7_20
LBB7_3:
	mov.16b	v9, v4
	mov.16b	v25, v1
	mov.16b	v24, v2
	mov.16b	v23, v3
	tbnz	w21, #0, LBB7_10
	fmov	d0, #1.00000000
	cmp	x23, x24
	b.lo	LBB7_15
LBB7_5:
	ldr	q5, [x19, #336]
	ldr	q1, [sp, #256]
	fmul.2d	v1, v1, v5
	str	q1, [x19, #336]
	fcmp	d10, #0.0
	b.le	LBB7_7
LBB7_6:
	fmul	d2, d10, d1
	fsub	d2, d21, d2
	fmul	d1, d2, d1
	str	d1, [x19, #336]
LBB7_7:
	ldp	q4, q3, [sp, #336]
	fmul.2d	v1, v4, v25
	fmul.2d	v2, v3, v19
	fadd.2d	v1, v1, v2
	fmul.2d	v2, v4, v19
	fmul.2d	v3, v3, v25
	fsub.2d	v19, v2, v3
	ldp	q6, q4, [sp, #304]
	fmul.2d	v2, v4, v24
	fmul.2d	v3, v6, v18
	fadd.2d	v2, v2, v3
	fmul.2d	v3, v4, v18
	fmul.2d	v4, v6, v24
	fsub.2d	v18, v3, v4
	ldp	q7, q6, [sp, #208]
	fmul.2d	v3, v6, v23
	fmul.2d	v4, v7, v20
	fadd.2d	v3, v3, v4
	fmul.2d	v4, v6, v20
	fmul.2d	v6, v7, v23
	fsub.2d	v20, v4, v6
	fmul	d4, d13, d9
	fmul	d6, d14, d12
	fadd	d4, d6, d4
	fmul	d6, d13, d12
	fmul	d7, d14, d9
	fsub	d12, d6, d7
	cbz	x23, LBB7_2
	and	x8, x23, #0x3ff
	cbnz	x8, LBB7_2
	fmul.2d	v6, v1, v1
	fmul.2d	v7, v19, v19
	fadd.2d	v6, v6, v7
	fsqrt.2d	v6, v6
	fmov.2d	v22, #1.00000000
	fdiv.2d	v6, v22, v6
	fmul.2d	v1, v1, v6
	fmul.2d	v19, v19, v6
	fmul.2d	v6, v2, v2
	fmul.2d	v7, v18, v18
	fadd.2d	v6, v6, v7
	fsqrt.2d	v6, v6
	fdiv.2d	v6, v22, v6
	fmul.2d	v2, v2, v6
	fmul.2d	v18, v18, v6
	fmul.2d	v6, v3, v3
	fmul.2d	v7, v20, v20
	fadd.2d	v6, v6, v7
	fsqrt.2d	v6, v6
	fdiv.2d	v6, v22, v6
	fmul.2d	v3, v3, v6
	fmul.2d	v20, v20, v6
	fmul	d6, d4, d4
	fmul	d7, d12, d12
	fadd	d6, d6, d7
	fsqrt	d6, d6
	fdiv	d6, d21, d6
	fmul	d4, d4, d6
	fmul	d12, d12, d6
	b	LBB7_2
LBB7_10:
	fadd	d11, d11, d21
	str	d11, [x19, #544]
	tbnz	w28, #0, LBB7_13
	ldr	d1, [sp, #88]
	fcmp	d11, d1
	b.le	LBB7_14
	strb	w25, [x19, #561]
LBB7_13:
	ldr	q0, [x19, #336]
	ldr	q1, [sp, #128]
	fmul.2d	v0, v1, v0
	str	q0, [x19, #336]
	ldp	q0, q1, [sp, #96]
	fmul.2d	v17, v1, v17
	fmul.2d	v16, v0, v16
	fmul	d8, d15, d8
	mov	w28, #1
	fmov	d0, #1.00000000
	cmp	x23, x24
	b.hs	LBB7_5
	b	LBB7_15
LBB7_14:
	ldr	d0, [sp]
	fnmul	d0, d11, d0
	fdiv	d0, d0, d1
	mov.16b	v15, v1
	stp	q17, q16, [x29, #-192]
	stp	q19, q18, [x29, #-224]
	stp	q23, q20, [x29, #-256]
	stp	q25, q24, [sp, #368]
	bl	_exp
	str	q0, [sp, #64]
	ldr	d0, [sp, #56]
	fnmul	d0, d11, d0
	fdiv	d0, d0, d15
	bl	_exp
	ldr	q1, [x19, #336]
	ldr	q2, [sp, #64]
	mov.d	v2[1], v0[0]
	fmul.2d	v0, v2, v1
	str	q0, [x19, #336]
	ldr	d0, [sp, #48]
	fnmul	d0, d11, d0
	fdiv	d0, d0, d15
	bl	_exp
	str	q0, [sp, #64]
	ldr	d0, [sp, #40]
	fnmul	d0, d11, d0
	fdiv	d0, d0, d15
	bl	_exp
	ldr	q1, [sp, #64]
	mov.d	v1[1], v0[0]
	ldur	q0, [x29, #-192]
	fmul.2d	v0, v1, v0
	stur	q0, [x29, #-192]
	ldr	d0, [sp, #32]
	fnmul	d0, d11, d0
	fdiv	d0, d0, d15
	bl	_exp
	str	q0, [sp, #64]
	ldr	d0, [sp, #24]
	fnmul	d0, d11, d0
	fdiv	d0, d0, d15
	bl	_exp
	ldr	q1, [sp, #64]
	mov.d	v1[1], v0[0]
	ldur	q0, [x29, #-176]
	fmul.2d	v0, v1, v0
	stur	q0, [x29, #-176]
	ldr	d0, [sp, #16]
	fnmul	d0, d11, d0
	fdiv	d0, d0, d15
	ldr	d15, [sp, #8]
	bl	_exp
	ldp	q25, q24, [sp, #368]
	ldp	q23, q20, [x29, #-256]
	fmov	d21, #1.00000000
	ldp	q19, q18, [x29, #-224]
	ldp	q17, q16, [x29, #-192]
	mov	w28, #0
	fmul	d8, d8, d0
	fmov	d0, #1.00000000
	cmp	x23, x24
	b.hs	LBB7_5
LBB7_15:
	stp	q25, q24, [sp, #368]
	stp	q23, q20, [x29, #-256]
	stp	q19, q18, [x29, #-224]
	stp	q17, q16, [x29, #-192]
	ucvtf	d0, x23
	ldr	d1, [sp, #152]
	fmul	d0, d1, d0
	bl	_cos
	fmov	d21, #1.00000000
	fsub	d0, d21, d0
	fmov	d1, #0.50000000
	fmul	d0, d0, d1
	fmov	d2, x26
	ldr	d1, [sp, #144]
	fcmp	d1, d2
	b.ls	LBB7_19
	fmov	d2, x27
	fcmp	d1, d2
	b.ge	LBB7_18
	bl	_pow
	fmov	d21, #1.00000000
	b	LBB7_19
LBB7_18:
	fmul	d0, d0, d0
LBB7_19:
	ldp	q17, q16, [x29, #-192]
	ldp	q19, q18, [x29, #-224]
	ldp	q23, q20, [x29, #-256]
	ldp	q25, q24, [sp, #368]
	ldr	q5, [x19, #336]
	ldr	q1, [sp, #256]
	fmul.2d	v1, v1, v5
	str	q1, [x19, #336]
	fcmp	d10, #0.0
	b.gt	LBB7_6
	b	LBB7_7
LBB7_20:
	stp	q17, q16, [x19, #352]
	stur	q19, [x19, #56]
	stp	q1, q2, [x19]
	stur	q18, [x19, #72]
	str	q3, [x19, #32]
	stur	q20, [x19, #88]
	str	d8, [x19, #384]
	str	x23, [x19, #504]
	str	d4, [x19, #48]
	str	d12, [x19, #104]
	add	sp, sp, #512
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
LBB7_21:
	ret
Lfunc_end7:
	