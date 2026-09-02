"""Generate tray-icon-warning.png: same as tray-icon-error.png but orange instead of red."""
import struct, zlib, os

def read_png(path):
    with open(path, 'rb') as f:
        data = f.read()
    # Parse IHDR
    ihdr_data = data[16:29]
    w, h = struct.unpack('>II', ihdr_data[:8])
    # Find IDAT chunks
    idat = b''
    pos = 8
    while pos < len(data):
        length = struct.unpack('>I', data[pos:pos+4])[0]
        ctype = data[pos+4:pos+8]
        chunk_data = data[pos+8:pos+8+length]
        if ctype == b'IDAT':
            idat += chunk_data
        pos += 8 + length + 4  # +4 for CRC
    raw = zlib.decompress(idat)
    return w, h, raw

def write_png(path, w, h, raw):
    def chunk(ctype, data):
        c = ctype + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    header = b'\x89PNG\r\n\x1a\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
    idat = chunk(b'IDAT', zlib.compress(raw))
    iend = chunk(b'IEND', b'')
    with open(path, 'wb') as f:
        f.write(header + ihdr + idat + iend)

def replace_color(raw, w, h, src_rgb, dst_rgb, tolerance=60):
    """Replace pixels matching src_rgb with dst_rgb (RGBA, 4 bytes per pixel)."""
    out = bytearray(raw)
    stride = 1 + w * 4  # filter byte + RGBA
    for y in range(h):
        row_start = y * stride + 1  # skip filter byte
        for x in range(w):
            px = row_start + x * 4
            r, g, b, a = out[px], out[px+1], out[px+2], out[px+3]
            if (abs(r - src_rgb[0]) <= tolerance and
                abs(g - src_rgb[1]) <= tolerance and
                abs(b - src_rgb[2]) <= tolerance and a > 0):
                out[px], out[px+1], out[px+2] = dst_rgb[0], dst_rgb[1], dst_rgb[2]
    return bytes(out)

icons_dir = os.path.dirname(os.path.abspath(__file__))
error_path = os.path.join(icons_dir, 'tray-icon-error.png')
warning_path = os.path.join(icons_dir, 'tray-icon-warning.png')

w, h, raw = read_png(error_path)
# Red circle ~(255,0,0) → Orange ~(245,158,11) / #F59E0B
# Also catch darker reds from anti-aliasing
new_raw = replace_color(raw, w, h, (255, 0, 0), (245, 158, 11), tolerance=80)
# Also replace lighter reds (255,50,50) etc
new_raw = replace_color(new_raw, w, h, (200, 0, 0), (200, 120, 10), tolerance=80)

write_png(warning_path, w, h, new_raw)
print(f'Created {warning_path} ({w}x{h})')
