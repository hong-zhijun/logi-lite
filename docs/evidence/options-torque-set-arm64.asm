/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent:
(__TEXT,__text) section
__ZN5devio38Feature2111SmartShiftWithTunableTorque21SetRatchetControlModeEjNS_4ByteES1_PjPS1_S3_:
1003f73a4:	ff 03 02 d1	sub	sp, sp, #0x80
1003f73a8:	fa 67 03 a9	stp	x26, x25, [sp, #0x30]
1003f73ac:	f8 5f 04 a9	stp	x24, x23, [sp, #0x40]
1003f73b0:	f6 57 05 a9	stp	x22, x21, [sp, #0x50]
1003f73b4:	f4 4f 06 a9	stp	x20, x19, [sp, #0x60]
1003f73b8:	fd 7b 07 a9	stp	x29, x30, [sp, #0x70]
1003f73bc:	fd c3 01 91	add	x29, sp, #0x70
1003f73c0:	f3 03 06 aa	mov	x19, x6
1003f73c4:	f4 03 05 aa	mov	x20, x5
1003f73c8:	f5 03 04 aa	mov	x21, x4
1003f73cc:	f6 03 03 aa	mov	x22, x3
1003f73d0:	f7 03 02 aa	mov	x23, x2
1003f73d4:	f8 03 01 aa	mov	x24, x1
1003f73d8:	f9 03 00 aa	mov	x25, x0
1003f73dc:	60 00 80 52	mov	w0, #0x3
1003f73e0:	26 b7 95 94	bl	0x102965078 ; symbol stub for: __Znwm
1003f73e4:	08 0c 00 91	add	x8, x0, #0x3
1003f73e8:	e8 23 02 a9	stp	x8, x8, [sp, #0x20]
1003f73ec:	ff 03 01 a9	stp	xzr, x0, [sp, #0x10]
1003f73f0:	ff 7f 00 a9	stp	xzr, xzr, [sp]
1003f73f4:	18 00 00 39	strb	w24, [x0]
1003f73f8:	17 04 00 39	strb	w23, [x0, #0x1]
1003f73fc:	16 08 00 39	strb	w22, [x0, #0x2]
1003f7400:	28 03 40 f9	ldr	x8, [x25]
1003f7404:	08 0d 40 f9	ldr	x8, [x8, #0x18]
1003f7408:	e2 63 00 91	add	x2, sp, #0x18
1003f740c:	e3 03 00 91	mov	x3, sp
1003f7410:	e0 03 19 aa	mov	x0, x25
1003f7414:	41 00 80 52	mov	w1, #0x2
1003f7418:	04 00 80 52	mov	w4, #0x0
1003f741c:	00 01 3f d6	blr	x8
1003f7420:	a0 01 00 36	tbz	w0, #0x0, 0x1003f7454
1003f7424:	e8 27 40 a9	ldp	x8, x9, [sp]
1003f7428:	29 01 08 cb	sub	x9, x9, x8
1003f742c:	3f 09 00 f1	cmp	x9, #0x2
1003f7430:	48 03 00 54	b.hi	0x1003f7498
1003f7434:	74 24 f8 97	bl	__ZTWN5devio7errcodeE
1003f7438:	48 00 80 52	mov	w8, #0x2
1003f743c:	08 00 00 b9	str	w8, [x0]
1003f7440:	81 24 f8 97	bl	__ZTWN5devio11errcode_strE
1003f7444:	1f 00 00 f9	str	xzr, [x0]
1003f7448:	77 24 f8 97	bl	__ZTWN5devio11errcode_sysE
1003f744c:	1f 00 00 f9	str	xzr, [x0]
1003f7450:	6d 24 f8 97	bl	__ZTWN5devio7errcodeE
1003f7454:	13 00 80 52	mov	w19, #0x0
1003f7458:	e0 03 40 f9	ldr	x0, [sp]
1003f745c:	60 00 00 b4	cbz	x0, 0x1003f7468
1003f7460:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f7464:	fc b6 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7468:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f746c:	60 00 00 b4	cbz	x0, 0x1003f7478
1003f7470:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f7474:	f8 b6 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7478:	e0 03 13 aa	mov	x0, x19
1003f747c:	fd 7b 47 a9	ldp	x29, x30, [sp, #0x70]
1003f7480:	f4 4f 46 a9	ldp	x20, x19, [sp, #0x60]
1003f7484:	f6 57 45 a9	ldp	x22, x21, [sp, #0x50]
1003f7488:	f8 5f 44 a9	ldp	x24, x23, [sp, #0x40]
1003f748c:	fa 67 43 a9	ldp	x26, x25, [sp, #0x30]
1003f7490:	ff 03 02 91	add	sp, sp, #0x80
1003f7494:	c0 03 5f d6	ret
1003f7498:	75 00 00 b4	cbz	x21, 0x1003f74a4
1003f749c:	09 01 40 39	ldrb	w9, [x8]
1003f74a0:	a9 02 00 b9	str	w9, [x21]
1003f74a4:	74 00 00 b4	cbz	x20, 0x1003f74b0
1003f74a8:	09 05 40 39	ldrb	w9, [x8, #0x1]
1003f74ac:	89 02 00 39	strb	w9, [x20]
1003f74b0:	73 00 00 b4	cbz	x19, 0x1003f74bc
1003f74b4:	08 09 40 39	ldrb	w8, [x8, #0x2]
1003f74b8:	68 02 00 39	strb	w8, [x19]
1003f74bc:	33 00 80 52	mov	w19, #0x1
1003f74c0:	e0 03 40 f9	ldr	x0, [sp]
1003f74c4:	e0 fc ff b5	cbnz	x0, 0x1003f7460
1003f74c8:	e8 ff ff 17	b	0x1003f7468
1003f74cc:	f3 03 00 aa	mov	x19, x0
1003f74d0:	e0 03 40 f9	ldr	x0, [sp]
1003f74d4:	a0 00 00 b5	cbnz	x0, 0x1003f74e8
1003f74d8:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f74dc:	e0 00 00 b5	cbnz	x0, 0x1003f74f8
1003f74e0:	e0 03 13 aa	mov	x0, x19
1003f74e4:	4c b0 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
1003f74e8:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f74ec:	da b6 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f74f0:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f74f4:	60 ff ff b4	cbz	x0, 0x1003f74e0
1003f74f8:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f74fc:	d6 b6 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7500:	e0 03 13 aa	mov	x0, x19
1003f7504:	44 b0 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
