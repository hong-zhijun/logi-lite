/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent:
(__TEXT,__text) section
__ZN5devio21Feature2110SmartShift21SetRatchetControlModeEjNS_4ByteES1_PjPS1_S3_:
1003f688c:	ff c3 01 d1	sub	sp, sp, #0x70
1003f6890:	f8 5f 03 a9	stp	x24, x23, [sp, #0x30]
1003f6894:	f6 57 04 a9	stp	x22, x21, [sp, #0x40]
1003f6898:	f4 4f 05 a9	stp	x20, x19, [sp, #0x50]
1003f689c:	fd 7b 06 a9	stp	x29, x30, [sp, #0x60]
1003f68a0:	fd 83 01 91	add	x29, sp, #0x60
1003f68a4:	f3 03 05 aa	mov	x19, x5
1003f68a8:	f4 03 04 aa	mov	x20, x4
1003f68ac:	f5 03 02 aa	mov	x21, x2
1003f68b0:	f6 03 01 aa	mov	x22, x1
1003f68b4:	f7 03 00 aa	mov	x23, x0
1003f68b8:	40 00 80 52	mov	w0, #0x2
1003f68bc:	ef b9 95 94	bl	0x102965078 ; symbol stub for: __Znwm
1003f68c0:	08 08 00 91	add	x8, x0, #0x2
1003f68c4:	e8 23 02 a9	stp	x8, x8, [sp, #0x20]
1003f68c8:	ff 03 01 a9	stp	xzr, x0, [sp, #0x10]
1003f68cc:	ff 7f 00 a9	stp	xzr, xzr, [sp]
1003f68d0:	16 00 00 39	strb	w22, [x0]
1003f68d4:	15 04 00 39	strb	w21, [x0, #0x1]
1003f68d8:	e8 02 40 f9	ldr	x8, [x23]
1003f68dc:	08 0d 40 f9	ldr	x8, [x8, #0x18]
1003f68e0:	e2 63 00 91	add	x2, sp, #0x18
1003f68e4:	e3 03 00 91	mov	x3, sp
1003f68e8:	e0 03 17 aa	mov	x0, x23
1003f68ec:	21 00 80 52	mov	w1, #0x1
1003f68f0:	04 00 80 52	mov	w4, #0x0
1003f68f4:	00 01 3f d6	blr	x8
1003f68f8:	a0 01 00 36	tbz	w0, #0x0, 0x1003f692c
1003f68fc:	e8 27 40 a9	ldp	x8, x9, [sp]
1003f6900:	29 01 08 cb	sub	x9, x9, x8
1003f6904:	3f 05 00 f1	cmp	x9, #0x1
1003f6908:	28 03 00 54	b.hi	0x1003f696c
1003f690c:	3e 27 f8 97	bl	__ZTWN5devio7errcodeE
1003f6910:	48 00 80 52	mov	w8, #0x2
1003f6914:	08 00 00 b9	str	w8, [x0]
1003f6918:	4b 27 f8 97	bl	__ZTWN5devio11errcode_strE
1003f691c:	1f 00 00 f9	str	xzr, [x0]
1003f6920:	41 27 f8 97	bl	__ZTWN5devio11errcode_sysE
1003f6924:	1f 00 00 f9	str	xzr, [x0]
1003f6928:	37 27 f8 97	bl	__ZTWN5devio7errcodeE
1003f692c:	13 00 80 52	mov	w19, #0x0
1003f6930:	e0 03 40 f9	ldr	x0, [sp]
1003f6934:	60 00 00 b4	cbz	x0, 0x1003f6940
1003f6938:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f693c:	c6 b9 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f6940:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f6944:	60 00 00 b4	cbz	x0, 0x1003f6950
1003f6948:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f694c:	c2 b9 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f6950:	e0 03 13 aa	mov	x0, x19
1003f6954:	fd 7b 46 a9	ldp	x29, x30, [sp, #0x60]
1003f6958:	f4 4f 45 a9	ldp	x20, x19, [sp, #0x50]
1003f695c:	f6 57 44 a9	ldp	x22, x21, [sp, #0x40]
1003f6960:	f8 5f 43 a9	ldp	x24, x23, [sp, #0x30]
1003f6964:	ff c3 01 91	add	sp, sp, #0x70
1003f6968:	c0 03 5f d6	ret
1003f696c:	74 00 00 b4	cbz	x20, 0x1003f6978
1003f6970:	09 01 40 39	ldrb	w9, [x8]
1003f6974:	89 02 00 b9	str	w9, [x20]
1003f6978:	73 00 00 b4	cbz	x19, 0x1003f6984
1003f697c:	08 05 40 39	ldrb	w8, [x8, #0x1]
1003f6980:	68 02 00 39	strb	w8, [x19]
1003f6984:	33 00 80 52	mov	w19, #0x1
1003f6988:	e0 03 40 f9	ldr	x0, [sp]
1003f698c:	60 fd ff b5	cbnz	x0, 0x1003f6938
1003f6990:	ec ff ff 17	b	0x1003f6940
1003f6994:	f3 03 00 aa	mov	x19, x0
1003f6998:	e0 03 40 f9	ldr	x0, [sp]
1003f699c:	a0 00 00 b5	cbnz	x0, 0x1003f69b0
1003f69a0:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f69a4:	e0 00 00 b5	cbnz	x0, 0x1003f69c0
1003f69a8:	e0 03 13 aa	mov	x0, x19
1003f69ac:	1a b3 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
1003f69b0:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f69b4:	a8 b9 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f69b8:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f69bc:	60 ff ff b4	cbz	x0, 0x1003f69a8
1003f69c0:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f69c4:	a4 b9 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f69c8:	e0 03 13 aa	mov	x0, x19
1003f69cc:	12 b3 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
