/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent:
(__TEXT,__text) section
__ZN5devio38Feature2111SmartShiftWithTunableTorque15GetCapabilitiesEPbPNS_4ByteES3_S3_S3_:
1003f7110:	ff c3 01 d1	sub	sp, sp, #0x70
1003f7114:	f8 5f 03 a9	stp	x24, x23, [sp, #0x30]
1003f7118:	f6 57 04 a9	stp	x22, x21, [sp, #0x40]
1003f711c:	f4 4f 05 a9	stp	x20, x19, [sp, #0x50]
1003f7120:	fd 7b 06 a9	stp	x29, x30, [sp, #0x60]
1003f7124:	fd 83 01 91	add	x29, sp, #0x60
1003f7128:	f3 03 05 aa	mov	x19, x5
1003f712c:	f4 03 04 aa	mov	x20, x4
1003f7130:	f5 03 03 aa	mov	x21, x3
1003f7134:	f6 03 02 aa	mov	x22, x2
1003f7138:	f7 03 01 aa	mov	x23, x1
1003f713c:	ff ff 01 a9	stp	xzr, xzr, [sp, #0x18]
1003f7140:	ff 17 00 f9	str	xzr, [sp, #0x28]
1003f7144:	ff 7f 00 a9	stp	xzr, xzr, [sp]
1003f7148:	ff 0b 00 f9	str	xzr, [sp, #0x10]
1003f714c:	08 00 40 f9	ldr	x8, [x0]
1003f7150:	08 0d 40 f9	ldr	x8, [x8, #0x18]
1003f7154:	e2 63 00 91	add	x2, sp, #0x18
1003f7158:	e3 03 00 91	mov	x3, sp
1003f715c:	01 00 80 52	mov	w1, #0x0
1003f7160:	04 00 80 52	mov	w4, #0x0
1003f7164:	00 01 3f d6	blr	x8
1003f7168:	a0 01 00 36	tbz	w0, #0x0, 0x1003f719c
1003f716c:	e8 27 40 a9	ldp	x8, x9, [sp]
1003f7170:	29 01 08 cb	sub	x9, x9, x8
1003f7174:	3f 0d 00 f1	cmp	x9, #0x3
1003f7178:	28 03 00 54	b.hi	0x1003f71dc
1003f717c:	22 25 f8 97	bl	__ZTWN5devio7errcodeE
1003f7180:	48 00 80 52	mov	w8, #0x2
1003f7184:	08 00 00 b9	str	w8, [x0]
1003f7188:	2f 25 f8 97	bl	__ZTWN5devio11errcode_strE
1003f718c:	1f 00 00 f9	str	xzr, [x0]
1003f7190:	25 25 f8 97	bl	__ZTWN5devio11errcode_sysE
1003f7194:	1f 00 00 f9	str	xzr, [x0]
1003f7198:	1b 25 f8 97	bl	__ZTWN5devio7errcodeE
1003f719c:	13 00 80 52	mov	w19, #0x0
1003f71a0:	e0 03 40 f9	ldr	x0, [sp]
1003f71a4:	60 00 00 b4	cbz	x0, 0x1003f71b0
1003f71a8:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f71ac:	aa b7 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f71b0:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f71b4:	60 00 00 b4	cbz	x0, 0x1003f71c0
1003f71b8:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f71bc:	a6 b7 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f71c0:	e0 03 13 aa	mov	x0, x19
1003f71c4:	fd 7b 46 a9	ldp	x29, x30, [sp, #0x60]
1003f71c8:	f4 4f 45 a9	ldp	x20, x19, [sp, #0x50]
1003f71cc:	f6 57 44 a9	ldp	x22, x21, [sp, #0x40]
1003f71d0:	f8 5f 43 a9	ldp	x24, x23, [sp, #0x30]
1003f71d4:	ff c3 01 91	add	sp, sp, #0x70
1003f71d8:	c0 03 5f d6	ret
1003f71dc:	97 00 00 b4	cbz	x23, 0x1003f71ec
1003f71e0:	09 01 40 39	ldrb	w9, [x8]
1003f71e4:	29 01 00 12	and	w9, w9, #0x1
1003f71e8:	e9 02 00 39	strb	w9, [x23]
1003f71ec:	76 00 00 b4	cbz	x22, 0x1003f71f8
1003f71f0:	09 05 40 39	ldrb	w9, [x8, #0x1]
1003f71f4:	c9 02 00 39	strb	w9, [x22]
1003f71f8:	75 00 00 b4	cbz	x21, 0x1003f7204
1003f71fc:	09 09 40 39	ldrb	w9, [x8, #0x2]
1003f7200:	a9 02 00 39	strb	w9, [x21]
1003f7204:	74 00 00 b4	cbz	x20, 0x1003f7210
1003f7208:	09 0d 40 39	ldrb	w9, [x8, #0x3]
1003f720c:	89 02 00 39	strb	w9, [x20]
1003f7210:	73 00 00 b4	cbz	x19, 0x1003f721c
1003f7214:	08 11 40 39	ldrb	w8, [x8, #0x4]
1003f7218:	68 02 00 39	strb	w8, [x19]
1003f721c:	33 00 80 52	mov	w19, #0x1
1003f7220:	e0 03 40 f9	ldr	x0, [sp]
1003f7224:	20 fc ff b5	cbnz	x0, 0x1003f71a8
1003f7228:	e2 ff ff 17	b	0x1003f71b0
1003f722c:	f3 03 00 aa	mov	x19, x0
1003f7230:	e0 03 40 f9	ldr	x0, [sp]
1003f7234:	a0 00 00 b5	cbnz	x0, 0x1003f7248
1003f7238:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f723c:	e0 00 00 b5	cbnz	x0, 0x1003f7258
1003f7240:	e0 03 13 aa	mov	x0, x19
1003f7244:	f4 b0 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
1003f7248:	e0 07 00 f9	str	x0, [sp, #0x8]
1003f724c:	82 b7 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7250:	e0 0f 40 f9	ldr	x0, [sp, #0x18]
1003f7254:	60 ff ff b4	cbz	x0, 0x1003f7240
1003f7258:	e0 13 00 f9	str	x0, [sp, #0x20]
1003f725c:	7e b7 95 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1003f7260:	e0 03 13 aa	mov	x0, x19
1003f7264:	ec b0 95 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
