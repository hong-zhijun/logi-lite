1009c6ea8:	ff 03 02 d1	sub	sp, sp, #0x80
1009c6eac:	f8 5f 04 a9	stp	x24, x23, [sp, #0x40]
1009c6eb0:	f6 57 05 a9	stp	x22, x21, [sp, #0x50]
1009c6eb4:	f4 4f 06 a9	stp	x20, x19, [sp, #0x60]
1009c6eb8:	fd 7b 07 a9	stp	x29, x30, [sp, #0x70]
1009c6ebc:	fd c3 01 91	add	x29, sp, #0x70
1009c6ec0:	f3 03 00 aa	mov	x19, x0
1009c6ec4:	20 00 40 f9	ldr	x0, [x1]
1009c6ec8:	c0 08 00 b4	cbz	x0, 0x1009c6fe0
1009c6ecc:	f4 03 01 aa	mov	x20, x1
1009c6ed0:	bf c3 1c b8	stur	wzr, [x29, #-0x34]
1009c6ed4:	ff e3 00 39	strb	wzr, [sp, #0x38]
1009c6ed8:	ff c3 00 39	strb	wzr, [sp, #0x30]
1009c6edc:	ff bf 00 39	strb	wzr, [sp, #0x2f]
1009c6ee0:	08 00 40 f9	ldr	x8, [x0]
1009c6ee4:	08 09 40 f9	ldr	x8, [x8, #0x10]
1009c6ee8:	e1 bf 00 91	add	x1, sp, #0x2f
1009c6eec:	02 00 80 d2	mov	x2, #0x0
1009c6ef0:	03 00 80 d2	mov	x3, #0x0
1009c6ef4:	04 00 80 d2	mov	x4, #0x0
1009c6ef8:	05 00 80 d2	mov	x5, #0x0
1009c6efc:	00 01 3f d6	blr	x8
1009c6f00:	80 02 40 f9	ldr	x0, [x20]
1009c6f04:	08 00 40 f9	ldr	x8, [x0]
1009c6f08:	08 0d 40 f9	ldr	x8, [x8, #0x18]
1009c6f0c:	a1 d3 00 d1	sub	x1, x29, #0x34
1009c6f10:	e2 e3 00 91	add	x2, sp, #0x38
1009c6f14:	e3 c3 00 91	add	x3, sp, #0x30
1009c6f18:	00 01 3f d6	blr	x8
1009c6f1c:	20 06 00 34	cbz	w0, 0x1009c6fe0
1009c6f20:	b4 c3 5c b8	ldur	w20, [x29, #-0x34]
1009c6f24:	f5 bf 40 39	ldrb	w21, [sp, #0x2f]
1009c6f28:	f6 e3 40 39	ldrb	w22, [sp, #0x38]
1009c6f2c:	f7 c3 40 39	ldrb	w23, [sp, #0x30]
1009c6f30:	e0 03 00 91	mov	x0, sp
1009c6f34:	01 00 80 d2	mov	x1, #0x0
1009c6f38:	e3 0c 6f 94	bl	__ZN4logi8protocol5mouse18SmartShiftSettingsC1EPN6google8protobuf5ArenaE
1009c6f3c:	9f 0a 00 71	cmp	w20, #0x2
1009c6f40:	e8 1f 80 52	mov	w8, #0xff
1009c6f44:	c4 02 48 7a	ccmp	w22, w8, #0x4, eq
1009c6f48:	e8 07 9f 1a	cset	w8, ne
1009c6f4c:	e8 53 00 39	strb	w8, [sp, #0x14]
1009c6f50:	48 06 80 52	mov	w8, #0x32
1009c6f54:	df ca 00 71	cmp	w22, #0x32
1009c6f58:	c8 32 88 1a	csel	w8, w22, w8, lo
1009c6f5c:	89 00 80 52	mov	w9, #0x4
1009c6f60:	1f 11 00 71	cmp	w8, #0x4
1009c6f64:	08 81 89 1a	csel	w8, w8, w9, hi
1009c6f68:	08 c9 00 51	sub	w8, w8, #0x32
1009c6f6c:	00 01 62 1e	scvtf	d0, w8
1009c6f70:	28 0b e8 d2	mov	x8, #0x4059000000000000
1009c6f74:	01 01 67 9e	fmov	d1, x8
1009c6f78:	00 08 61 1e	fmul	d0, d0, d1
1009c6f7c:	e8 08 f8 d2	mov	x8, #-0x3fb9000000000000
1009c6f80:	01 01 67 9e	fmov	d1, x8
1009c6f84:	00 18 61 1e	fdiv	d0, d0, d1
1009c6f88:	08 00 79 1e	fcvtzu	w8, d0
1009c6f8c:	e8 13 00 b9	str	w8, [sp, #0x10]
1009c6f90:	f4 5f 03 29	stp	w20, w23, [sp, #0x18]
1009c6f94:	f5 57 00 39	strb	w21, [sp, #0x15]
1009c6f98:	00 08 80 52	mov	w0, #0x40
1009c6f9c:	37 78 7e 94	bl	0x102965078 ; symbol stub for: __Znwm
1009c6fa0:	f4 03 00 aa	mov	x20, x0
1009c6fa4:	1f fc 00 a9	stp	xzr, xzr, [x0, #0x8]
1009c6fa8:	48 5b 01 90	adrp	x8, 11112 ; 0x10352e000
1009c6fac:	08 01 17 91	add	x8, x8, #0x5c0
1009c6fb0:	08 84 01 f8	str	x8, [x0], #0x18
1009c6fb4:	e1 03 00 91	mov	x1, sp
1009c6fb8:	de 0c 6f 94	bl	__ZN4logi8protocol5mouse18SmartShiftSettingsC1ERKS2_
1009c6fbc:	60 52 00 a9	stp	x0, x20, [x19]
1009c6fc0:	e0 03 00 91	mov	x0, sp
1009c6fc4:	05 0d 6f 94	bl	__ZN4logi8protocol5mouse18SmartShiftSettingsD1Ev
1009c6fc8:	fd 7b 47 a9	ldp	x29, x30, [sp, #0x70]
1009c6fcc:	f4 4f 46 a9	ldp	x20, x19, [sp, #0x60]
1009c6fd0:	f6 57 45 a9	ldp	x22, x21, [sp, #0x50]
1009c6fd4:	f8 5f 44 a9	ldp	x24, x23, [sp, #0x40]
1009c6fd8:	ff 03 02 91	add	sp, sp, #0x80
1009c6fdc:	c0 03 5f d6	ret
1009c6fe0:	7f 7e 00 a9	stp	xzr, xzr, [x19]
1009c6fe4:	fd 7b 47 a9	ldp	x29, x30, [sp, #0x70]
1009c6fe8:	f4 4f 46 a9	ldp	x20, x19, [sp, #0x60]
1009c6fec:	f6 57 45 a9	ldp	x22, x21, [sp, #0x50]
1009c6ff0:	f8 5f 44 a9	ldp	x24, x23, [sp, #0x40]
1009c6ff4:	ff 03 02 91	add	sp, sp, #0x80
1009c6ff8:	c0 03 5f d6	ret
1009c6ffc:	f3 03 00 aa	mov	x19, x0
1009c7000:	e0 03 14 aa	mov	x0, x20
1009c7004:	df 76 7e 94	bl	0x102964b80 ; symbol stub for: __ZNSt3__119__shared_weak_countD2Ev
1009c7008:	13 78 7e 94	bl	0x102965054 ; symbol stub for: __ZdlPv
1009c700c:	02 00 00 14	b	0x1009c7014
1009c7010:	f3 03 00 aa	mov	x19, x0
1009c7014:	e0 03 00 91	mov	x0, sp
1009c7018:	f0 0c 6f 94	bl	__ZN4logi8protocol5mouse18SmartShiftSettingsD1Ev
1009c701c:	e0 03 13 aa	mov	x0, x19
1009c7020:	7d 71 7e 94	bl	0x102963614 ; symbol stub for: __Unwind_Resume
