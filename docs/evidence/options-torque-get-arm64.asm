/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent:
(__TEXT,__text) section
__ZN5devio38Feature2111SmartShiftWithTunableTorque21GetRatchetControlModeEPjPNS_4ByteES3_:
1003f7270:	ff 83 01 d1	sub	sp, sp, #0x60
1003f7274:	f6 57 03 a9	stp	x22, x21, [sp, #0x30]
1003f7278:	f4 4f 04 a9	stp	x20, x19, [sp, #0x40]
1003f727c:	fd 7b 05 a9	stp	x29, x30, [sp, #0x50]
1003f7280:	fd 43 01 91	add	x29, sp, #0x50
1003f7284:	f3 03 03 aa	mov	x19, x3
1003f7288:	f4 03 02 aa	mov	x20, x2
1003f728c:	f5 03 01 aa	mov	x21, x1
1003f7290:	ff ff 01 a9	stp	xzr, xzr, [sp, #0x18]
1003f7294:	ff 17 00 f9	str	xzr, [sp, #0x28]
1003f7298:	ff 7f 00 a9	stp	xzr, xzr, [sp]
1003f729c:	ff 0b 00 f9	str	xzr, [sp, #0x10]
1003f72a0:	08 00 40 f9	ldr	x8, [x0]
1003f72a4:	08 0d 40 f9	ldr	x8, [x8, #0x18]
1003f72a8:	e2 63 00 91	add	x2, sp, #0x18
1003f72ac:	e3 03 00 91	mov	x3, sp
1003f72b0:	21 00 80 52	mov	w1, #0x1
1003f72b4:	04 00 80 52	mov	w4, #0x0
1003f72b8:	00 01 3f d6	blr	x8
1003f72bc:	a0 01 00 36	tbz	w0, #0x0, 0x1003f72f0
1003f72c0:	e8 27 40 a9	ldp	x8, x9, [sp]
1003f72c4:	29 01 08 cb	sub	x9, x9, x8
1003f72c8:	3f 09 00 f1	cmp	x9, #0x2
1003f72cc:	08 03 00 54	b.hi	0x1003f732c
1003f72d0:	cd 24 f8 97	bl	__ZTWN5devio7errcodeE
1003f72d4:	48 00 80 52	mov	w8, #0x2
1003f72d8:	08 00 00 b9	str	w8, [x0]
1003f72dc:	da 24 f8 97	bl	__ZTWN5devio11errcode_strE
1003f72e0:	1f 00 00 f9	str	xzr, [x0]
1003f72e4:	d0 24 f8 97	bl	__ZTWN5devio11errcode_sysE
1003f72e8:	1f 00 00 f9	str	xzr, [x0]
1003f72ec:	c6 24 f8 97	bl	__ZTWN5devio7errcodeE
1003f72f0:	13 00 80 52	mov	w19, #0x0
1003f72f4:	e0 03 40 f9	ldr	x0, [sp]
1003f72f8:	60 00 00 b4	cbz	x0, 0x1003f7304
1003f72fc:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f7300:	55 b7 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7304:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f7308:	60 00 00 b4	cbz	x0, 0x1003f7314
1003f730c:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f7310:	51 b7 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7314:	e0 03 13 aa	mov	x0, x19
1003f7318:	fd 7b 45 a9	ldp	x29, x30, [sp, #0x50]
1003f731c:	f4 4f 44 a9	ldp	x20, x19, [sp, #0x40]
1003f7320:	f6 57 43 a9	ldp	x22, x21, [sp, #0x30]
1003f7324:	ff 83 01 91	add	sp, sp, #0x60
1003f7328:	c0 03 5f d6	ret
1003f732c:	75 00 00 b4	cbz	x21, 0x1003f7338
1003f7330:	09 01 40 39	ldrb	w9, [x8]
1003f7334:	a9 02 00 b9	str	w9, [x21]
1003f7338:	74 00 00 b4	cbz	x20, 0x1003f7344
1003f733c:	09 05 40 39	ldrb	w9, [x8, #0x1]
1003f7340:	89 02 00 39	strb	w9, [x20]
1003f7344:	73 00 00 b4	cbz	x19, 0x1003f7350
1003f7348:	08 09 40 39	ldrb	w8, [x8, #0x2]
1003f734c:	68 02 00 39	strb	w8, [x19]
1003f7350:	33 00 80 52	mov	w19, #0x1
1003f7354:	e0 03 40 f9	ldr	x0, [sp]
1003f7358:	20 fd ff b5	cbnz	x0, 0x1003f72fc
1003f735c:	ea ff ff 17	b	0x1003f7304
1003f7360:	f3 03 00 aa	mov	x19, x0
1003f7364:	e0 03 40 f9	ldr	x0, [sp]
1003f7368:	a0 00 00 b5	cbnz	x0, 0x1003f737c
1003f736c:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f7370:	e0 00 00 b5	cbnz	x0, 0x1003f738c
1003f7374:	e0 03 13 aa	mov	x0, x19
1003f7378:	a7 b0 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
1003f737c:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f7380:	35 b7 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7384:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f7388:	60 ff ff b4	cbz	x0, 0x1003f7374
1003f738c:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f7390:	31 b7 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7394:	e0 03 13 aa	mov	x0, x19
1003f7398:	9f b0 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
