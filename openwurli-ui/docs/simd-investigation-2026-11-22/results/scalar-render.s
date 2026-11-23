_scalar_render:
Lfunc_begin6:
	.cfi_startproc
	cbz	x2, LBB6_20
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
	sub	sp, sp, #496
	mov	x20, x1
	mov	x19, x0
	ldrb	w21, [x0, #560]
	lsl	x22, x2, #3
	ldr	d19, [x0, #536]
	ldp	d1, d2, [x0, #128]
	ldp	d3, d0, [x0, #272]
	stp	d3, d1, [sp, #48]
	stp	d0, d2, [sp, #104]
	ldp	d1, d2, [x0, #416]
	str	d1, [sp, #40]
	ldr	d0, [x0, #496]
	stp	d0, d2, [sp, #88]
	ldr	d1, [x0, #520]
	ldr	d0, [x0, #528]
	stp	d0, d1, [sp, #120]
	ldp	d1, d0, [x0, #16]
	stp	d0, d1, [sp, #360]
	ldp	d1, d0, [x0, #32]
	stp	d0, d1, [sp, #344]
	ldp	d0, d1, [x0, #96]
	stp	d0, d1, [sp, #328]
	ldp	d1, d0, [x0, #160]
	stp	d0, d1, [sp, #312]
	ldp	d1, d0, [x0, #176]
	stp	d0, d1, [sp, #296]
	ldp	d0, d1, [x0, #240]
	stp	d0, d1, [sp, #280]
	ldp	d1, d0, [x0, #304]
	stp	d0, d1, [sp, #264]
	ldp	d1, d0, [x0, #320]
	stp	d0, d1, [sp, #248]
	ldp	d0, d1, [x0, #384]
	stp	d0, d1, [sp, #232]
	ldp	d1, d0, [x0, #448]
	stp	d0, d1, [sp, #216]
	ldp	d1, d0, [x0, #464]
	stp	d0, d1, [sp, #200]
	ldp	x24, x23, [x0, #504]
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
	ldr	d1, [x0, #552]
	ldr	d20, [x0, #544]
	ldp	d8, d2, [x0, #48]
	ldp	d0, d10, [x0, #112]
	stp	d0, d1, [sp, #184]
	ldp	d15, d0, [x0, #192]
	stp	d0, d2, [sp, #24]
	ldp	d1, d9, [x0, #256]
	ldp	d14, d2, [x0, #336]
	ldp	d0, d11, [x0, #400]
	stp	d0, d1, [sp, #168]
	ldp	d12, d0, [x0, #480]
	stp	d0, d2, [sp, #8]
	ldp	d13, d21, [x0]
	ldp	d1, d31, [x0, #64]
	ldp	d23, d2, [x0, #80]
	ldp	d16, d22, [x0, #144]
	ldp	d0, d17, [x0, #208]
	stp	d0, d1, [sp, #72]
	ldp	d25, d0, [x0, #224]
	stp	d0, d2, [sp, #152]
	ldp	d18, d24, [x0, #288]
	ldp	d0, d30, [x0, #352]
	str	d0, [sp, #64]
	ldp	d26, d0, [x0, #368]
	str	d0, [sp, #144]
	fmov	d28, #1.00000000
	ldp	d29, d27, [x0, #432]
	stur	d19, [x29, #-160]
	b	LBB6_4
LBB6_2:
	mov.16b	v29, v7
	mov.16b	v30, v6
	mov.16b	v18, v5
	mov.16b	v17, v4
	mov.16b	v16, v3
	mov.16b	v31, v2
	mov.16b	v13, v1
LBB6_3:
	ldr	d1, [x20]
	fadd	d0, d0, d1
	str	d0, [x20], #8
	ldr	d0, [sp, #184]
	fmul	d10, d0, d10
	ldr	d0, [sp, #296]
	fmul	d15, d0, d15
	ldr	d0, [sp, #176]
	fmul	d9, d0, d9
	ldr	d0, [sp, #248]
	fmul	d14, d0, d14
	ldr	d0, [sp, #168]
	fmul	d11, d0, d11
	ldr	d0, [sp, #200]
	fmul	d12, d0, d12
	add	x24, x24, #1
	subs	x22, x22, #8
	b.eq	LBB6_19
LBB6_4:
	tbz	w21, #0, LBB6_9
	fadd	d20, d20, d28
	str	d20, [x19, #544]
	tbnz	w28, #0, LBB6_8
	fcmp	d20, d19
	b.le	LBB6_13
	strb	w25, [x19, #561]
LBB6_8:
	ldr	d0, [sp, #80]
	fmul	d8, d0, d8
	ldr	d0, [sp, #112]
	fmul	d10, d0, d10
	ldr	d0, [sp, #72]
	fmul	d15, d0, d15
	ldr	d0, [sp, #104]
	fmul	d9, d0, d9
	ldr	d0, [sp, #64]
	fmul	d14, d0, d14
	ldp	d0, d1, [sp, #88]
	fmul	d11, d1, d11
	mov	w28, #1
	fmul	d12, d0, d12
LBB6_9:
	fmov	d0, #1.00000000
	cmp	x24, x23
	b.hs	LBB6_16
LBB6_10:
	str	d13, [sp, #376]
	stp	d31, d16, [x29, #-256]
	stp	d17, d18, [x29, #-240]
	stp	d30, d29, [x29, #-224]
	stp	d27, d26, [x29, #-208]
	stp	d25, d24, [x29, #-192]
	stp	d23, d22, [x29, #-176]
	str	d12, [sp, #136]
	mov.16b	v12, v11
	mov.16b	v11, v14
	mov.16b	v14, v9
	mov.16b	v9, v15
	mov.16b	v15, v10
	mov.16b	v10, v8
	mov.16b	v8, v21
	mov.16b	v13, v20
	ucvtf	d0, x24
	ldr	d1, [sp, #128]
	fmul	d0, d1, d0
	bl	_cos
	fmov	d28, #1.00000000
	fsub	d0, d28, d0
	fmov	d1, #0.50000000
	fmul	d0, d0, d1
	fmov	d2, x26
	ldr	d1, [sp, #120]
	fcmp	d1, d2
	b.ls	LBB6_15
	fmov	d2, x27
	fcmp	d1, d2
	b.ge	LBB6_14
	bl	_pow
	fmov	d28, #1.00000000
	b	LBB6_15
LBB6_13:
	ldr	d0, [sp, #32]
	fnmul	d0, d20, d0
	fdiv	d0, d0, d19
	str	d13, [sp, #376]
	mov.16b	v13, v20
	str	d21, [sp, #136]
	stp	d23, d22, [x29, #-176]
	stp	d25, d24, [x29, #-192]
	stp	d27, d26, [x29, #-208]
	stp	d30, d29, [x29, #-224]
	stp	d17, d18, [x29, #-240]
	stp	d31, d16, [x29, #-256]
	bl	_exp
	fmul	d8, d8, d0
	ldr	d0, [sp, #56]
	fnmul	d0, d13, d0
	ldur	d1, [x29, #-160]
	fdiv	d0, d0, d1
	bl	_exp
	fmul	d10, d10, d0
	ldr	d0, [sp, #24]
	fnmul	d0, d13, d0
	ldur	d1, [x29, #-160]
	fdiv	d0, d0, d1
	bl	_exp
	fmul	d15, d15, d0
	ldr	d0, [sp, #48]
	fnmul	d0, d13, d0
	ldur	d1, [x29, #-160]
	fdiv	d0, d0, d1
	bl	_exp
	fmul	d9, d9, d0
	ldr	d0, [sp, #16]
	fnmul	d0, d13, d0
	ldur	d1, [x29, #-160]
	fdiv	d0, d0, d1
	bl	_exp
	fmul	d14, d14, d0
	ldr	d0, [sp, #40]
	fnmul	d0, d13, d0
	ldur	d1, [x29, #-160]
	fdiv	d0, d0, d1
	bl	_exp
	fmul	d11, d11, d0
	ldr	d0, [sp, #8]
	fnmul	d0, d13, d0
	ldur	d1, [x29, #-160]
	fdiv	d0, d0, d1
	bl	_exp
	ldp	d31, d16, [x29, #-256]
	ldp	d17, d18, [x29, #-240]
	ldp	d30, d29, [x29, #-224]
	fmov	d28, #1.00000000
	ldp	d27, d26, [x29, #-208]
	ldp	d25, d24, [x29, #-192]
	ldp	d23, d22, [x29, #-176]
	ldr	d21, [sp, #136]
	mov.16b	v20, v13
	ldr	d13, [sp, #376]
	ldur	d19, [x29, #-160]
	mov	w28, #0
	fmul	d12, d12, d0
	fmov	d0, #1.00000000
	cmp	x24, x23
	b.lo	LBB6_10
	b	LBB6_16
LBB6_14:
	fmul	d0, d0, d0
LBB6_15:
	ldp	d22, d19, [x29, #-168]
	mov.16b	v20, v13
	mov.16b	v21, v8
	mov.16b	v8, v10
	mov.16b	v10, v15
	mov.16b	v15, v9
	mov.16b	v9, v14
	mov.16b	v14, v11
	mov.16b	v11, v12
	ldr	d12, [sp, #136]
	ldp	d24, d23, [x29, #-184]
	ldp	d26, d25, [x29, #-200]
	ldp	d29, d27, [x29, #-216]
	ldp	d18, d30, [x29, #-232]
	ldp	d16, d17, [x29, #-248]
	ldur	d31, [x29, #-256]
	ldr	d13, [sp, #376]
LBB6_16:
	ldp	d1, d5, [sp, #352]
	fmul	d1, d1, d13
	fmul	d1, d1, d0
	fmul	d1, d8, d1
	movi.2d	v2, #0000000000000000
	fadd	d2, d1, d2
	ldr	d4, [sp, #368]
	fmul	d1, d4, d13
	fmul	d3, d5, d21
	fadd	d1, d3, d1
	fmul	d3, d4, d21
	fmul	d4, d5, d13
	fsub	d21, d3, d4
	ldp	d5, d3, [sp, #328]
	fmul	d3, d3, d31
	fmul	d3, d3, d0
	fmul	d3, d10, d3
	fadd	d3, d3, d2
	ldr	d6, [sp, #160]
	fmul	d2, d6, d31
	fmul	d4, d5, d23
	fadd	d2, d4, d2
	fmul	d4, d6, d23
	fmul	d5, d5, d31
	fsub	d23, d4, d5
	ldp	d4, d7, [sp, #304]
	fmul	d4, d4, d16
	fmul	d4, d4, d0
	fmul	d4, d15, d4
	fadd	d4, d4, d3
	ldr	d6, [sp, #320]
	fmul	d3, d6, d16
	fmul	d5, d7, d22
	fadd	d3, d5, d3
	fmul	d5, d6, d22
	fmul	d6, d7, d16
	fsub	d22, d5, d6
	ldp	d7, d5, [sp, #280]
	fmul	d5, d5, d17
	fmul	d5, d5, d0
	fmul	d5, d9, d5
	fadd	d5, d5, d4
	ldr	d16, [sp, #152]
	fmul	d4, d16, d17
	fmul	d6, d7, d25
	fadd	d4, d6, d4
	fmul	d6, d16, d25
	fmul	d7, d7, d17
	fsub	d25, d6, d7
	ldp	d6, d17, [sp, #256]
	fmul	d6, d6, d18
	fmul	d6, d6, d0
	fmul	d6, d14, d6
	fadd	d6, d6, d5
	ldr	d16, [sp, #272]
	fmul	d5, d16, d18
	fmul	d7, d17, d24
	fadd	d5, d7, d5
	fmul	d7, d16, d24
	fmul	d16, d17, d18
	fsub	d24, d7, d16
	ldp	d17, d7, [sp, #232]
	fmul	d7, d7, d30
	fmul	d7, d7, d0
	fmul	d7, d11, d7
	fadd	d7, d7, d6
	ldr	d18, [sp, #144]
	fmul	d6, d18, d30
	fmul	d16, d17, d26
	fadd	d6, d16, d6
	fmul	d16, d18, d26
	fmul	d17, d17, d30
	fsub	d26, d16, d17
	ldp	d16, d18, [sp, #208]
	fmul	d16, d16, d29
	fmul	d0, d16, d0
	fmul	d0, d12, d0
	fadd	d0, d0, d7
	ldr	d17, [sp, #224]
	fmul	d7, d17, d29
	fmul	d16, d18, d27
	fadd	d7, d16, d7
	ldr	d16, [sp, #344]
	fmul	d16, d16, d8
	fmul	d17, d17, d27
	fmul	d18, d18, d29
	fsub	d27, d17, d18
	ldr	d18, [sp, #192]
	fmul	d17, d18, d16
	fsub	d17, d28, d17
	fmul	d17, d16, d17
	fcmp	d18, #0.0
	fcsel	d8, d17, d16, gt
	cbz	x24, LBB6_2
	and	x8, x24, #0x3ff
	cbnz	x8, LBB6_2
	fmul	d16, d1, d1
	fmul	d17, d21, d21
	fadd	d16, d16, d17
	fsqrt	d16, d16
	fdiv	d16, d28, d16
	fmul	d13, d1, d16
	fmul	d21, d21, d16
	fmul	d1, d2, d2
	fmul	d16, d23, d23
	fadd	d1, d1, d16
	fsqrt	d1, d1
	fdiv	d1, d28, d1
	fmul	d31, d2, d1
	fmul	d23, d23, d1
	fmul	d1, d3, d3
	fmul	d2, d22, d22
	fadd	d1, d1, d2
	fsqrt	d1, d1
	fdiv	d1, d28, d1
	fmul	d16, d3, d1
	fmul	d22, d22, d1
	fmul	d1, d4, d4
	fmul	d2, d25, d25
	fadd	d1, d1, d2
	fsqrt	d1, d1
	fdiv	d1, d28, d1
	fmul	d17, d4, d1
	fmul	d25, d25, d1
	fmul	d1, d5, d5
	fmul	d2, d24, d24
	fadd	d1, d1, d2
	fsqrt	d1, d1
	fdiv	d1, d28, d1
	fmul	d18, d5, d1
	fmul	d24, d24, d1
	fmul	d1, d6, d6
	fmul	d2, d26, d26
	fadd	d1, d1, d2
	fsqrt	d1, d1
	fdiv	d1, d28, d1
	fmul	d30, d6, d1
	fmul	d26, d26, d1
	fmul	d1, d7, d7
	fmul	d2, d27, d27
	fadd	d1, d1, d2
	fsqrt	d1, d1
	fdiv	d1, d28, d1
	fmul	d29, d7, d1
	fmul	d27, d27, d1
	b	LBB6_3
LBB6_19:
	str	d8, [x19, #48]
	str	d10, [x19, #120]
	str	d15, [x19, #192]
	str	d9, [x19, #264]
	str	d14, [x19, #336]
	str	d11, [x19, #408]
	str	d12, [x19, #480]
	str	x24, [x19, #504]
	stp	d13, d21, [x19]
	stp	d31, d23, [x19, #72]
	stp	d16, d22, [x19, #144]
	stp	d17, d25, [x19, #216]
	stp	d18, d24, [x19, #288]
	stp	d30, d26, [x19, #360]
	stp	d29, d27, [x19, #432]
	add	sp, sp, #496
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
LBB6_20:
	ret
Lfunc_end6:
	