/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent:
(__TEXT,__text) section
__ZN5devio21Feature2110SmartShift21GetRatchetControlModeEPjPNS_4ByteES3_:
1003f6770:	ff 43 01 d1	sub	sp, sp, #0x50
1003f6774:	f4 4f 03 a9	stp	x20, x19, [sp, #0x30]
1003f6778:	fd 7b 04 a9	stp	x29, x30, [sp, #0x40]
1003f677c:	fd 03 01 91	add	x29, sp, #0x40
1003f6780:	f3 03 02 aa	mov	x19, x2
1003f6784:	f4 03 01 aa	mov	x20, x1
1003f6788:	ff ff 01 a9	stp	xzr, xzr, [sp, #0x18]
1003f678c:	ff 17 00 f9	str	xzr, [sp, #0x28]
1003f6790:	ff 7f 00 a9	stp	xzr, xzr, [sp]
1003f6794:	ff 0b 00 f9	str	xzr, [sp, #0x10]
1003f6798:	08 00 40 f9	ldr	x8, [x0]
1003f679c:	08 0d 40 f9	ldr	x8, [x8, #0x18]
1003f67a0:	e2 63 00 91	add	x2, sp, #0x18
1003f67a4:	e3 03 00 91	mov	x3, sp
1003f67a8:	01 00 80 52	mov	w1, #0x0
1003f67ac:	04 00 80 52	mov	w4, #0x0
1003f67b0:	00 01 3f d6	blr	x8
1003f67b4:	a0 01 00 36	tbz	w0, #0x0, 0x1003f67e8
1003f67b8:	e8 27 40 a9	ldp	x8, x9, [sp]
1003f67bc:	29 01 08 cb	sub	x9, x9, x8
1003f67c0:	3f 05 00 f1	cmp	x9, #0x1
1003f67c4:	e8 02 00 54	b.hi	0x1003f6820
1003f67c8:	8f 27 f8 97	bl	__ZTWN5devio7errcodeE
1003f67cc:	48 00 80 52	mov	w8, #0x2
1003f67d0:	08 00 00 b9	str	w8, [x0]
1003f67d4:	9c 27 f8 97	bl	__ZTWN5devio11errcode_strE
1003f67d8:	1f 00 00 f9	str	xzr, [x0]
1003f67dc:	92 27 f8 97	bl	__ZTWN5devio11errcode_sysE
1003f67e0:	1f 00 00 f9	str	xzr, [x0]
1003f67e4:	88 27 f8 97	bl	__ZTWN5devio7errcodeE
1003f67e8:	13 00 80 52	mov	w19, #0x0
1003f67ec:	e0 03 40 f9	ldr	x0, [sp]
1003f67f0:	60 00 00 b4	cbz	x0, 0x1003f67fc
1003f67f4:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f67f8:	17 ba 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f67fc:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f6800:	60 00 00 b4	cbz	x0, 0x1003f680c
1003f6804:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f6808:	13 ba 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f680c:	e0 03 13 aa	mov	x0, x19
1003f6810:	fd 7b 44 a9	ldp	x29, x30, [sp, #0x40]
1003f6814:	f4 4f 43 a9	ldp	x20, x19, [sp, #0x30]
1003f6818:	ff 43 01 91	add	sp, sp, #0x50
1003f681c:	c0 03 5f d6	ret
1003f6820:	74 00 00 b4	cbz	x20, 0x1003f682c
1003f6824:	09 01 40 39	ldrb	w9, [x8]
1003f6828:	89 02 00 b9	str	w9, [x20]
1003f682c:	73 00 00 b4	cbz	x19, 0x1003f6838
1003f6830:	08 05 40 39	ldrb	w8, [x8, #0x1]
1003f6834:	68 02 00 39	strb	w8, [x19]
1003f6838:	33 00 80 52	mov	w19, #0x1
1003f683c:	e0 03 40 f9	ldr	x0, [sp]
1003f6840:	a0 fd ff b5	cbnz	x0, 0x1003f67f4
1003f6844:	ee ff ff 17	b	0x1003f67fc
1003f6848:	f3 03 00 aa	mov	x19, x0
1003f684c:	e0 03 40 f9	ldr	x0, [sp]
1003f6850:	a0 00 00 b5	cbnz	x0, 0x1003f6864
1003f6854:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f6858:	e0 00 00 b5	cbnz	x0, 0x1003f6874
1003f685c:	e0 03 13 aa	mov	x0, x19
1003f6860:	6d b3 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
1003f6864:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f6868:	fb b9 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f686c:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f6870:	60 ff ff b4	cbz	x0, 0x1003f685c
1003f6874:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f6878:	f7 b9 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f687c:	e0 03 13 aa	mov	x0, x19
1003f6880:	65 b3 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
