import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image


VGA_PALETTE = [
    (0x00, 0x00, 0x00),
    (0x00, 0x00, 0xAA),
    (0x00, 0xAA, 0x00),
    (0x00, 0xAA, 0xAA),
    (0xAA, 0x00, 0x00),
    (0xAA, 0x00, 0xAA),
    (0xAA, 0x55, 0x00),
    (0xAA, 0xAA, 0xAA),
    (0x55, 0x55, 0x55),
    (0x55, 0x55, 0xFF),
    (0x55, 0xFF, 0x55),
    (0x55, 0xFF, 0xFF),
    (0xFF, 0x55, 0x55),
    (0xFF, 0x55, 0xFF),
    (0xFF, 0xFF, 0x55),
    (0xFF, 0xFF, 0xFF),
]

TIMG_SIGNATURE = b"TIMG"
SVG_BROWSERS = [
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Convert png/jpg/svg files into Teddy-OS .timg images."
    )
    parser.add_argument("input", help="Source image path")
    parser.add_argument(
        "-o",
        "--output",
        help="Output .timg path (defaults to input name with .timg extension)",
    )
    parser.add_argument(
        "--max-width",
        type=int,
        default=128,
        help="Maximum output width in pixels",
    )
    parser.add_argument(
        "--max-height",
        type=int,
        default=96,
        help="Maximum output height in pixels",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    input_path = Path(args.input).expanduser().resolve()
    if not input_path.exists():
        print(f"input file not found: {input_path}", file=sys.stderr)
        return 1

    output_path = (
        Path(args.output).expanduser().resolve()
        if args.output
        else input_path.with_suffix(".timg")
    )

    image = load_image(input_path, args.max_width, args.max_height)
    image = fit_image(image, args.max_width, args.max_height)
    pixels = quantize_to_vga(image)
    encoded = encode_timg(image.width, image.height, pixels)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_bytes(encoded)
    print(f"wrote {output_path} ({image.width}x{image.height}, {len(encoded)} bytes)")
    return 0


def load_image(path: Path, max_width: int, max_height: int) -> Image.Image:
    suffix = path.suffix.lower()
    if suffix == ".svg":
        return rasterize_svg(path, max_width, max_height)
    return Image.open(path).convert("RGBA")


def rasterize_svg(path: Path, max_width: int, max_height: int) -> Image.Image:
    browser = find_browser()
    if browser is None:
        raise SystemExit(
            "SVG import needs Edge or Chrome installed for headless rasterization."
        )

    viewport_w = max(max_width * 2, 256)
    viewport_h = max(max_height * 2, 256)
    with tempfile.TemporaryDirectory() as temp_dir:
        temp = Path(temp_dir)
        screenshot = temp / "svg.png"
        html = temp / "viewer.html"
        svg_url = path.as_uri()
        html.write_text(
            "\n".join(
                [
                    "<!doctype html>",
                    "<html><body style='margin:0;background:#ffffff;display:flex;align-items:center;justify-content:center;width:100vw;height:100vh;'>",
                    f"<img src='{svg_url}' style='max-width:100vw;max-height:100vh;object-fit:contain;'>",
                    "</body></html>",
                ]
            ),
            encoding="utf-8",
        )
        command = [
            browser,
            "--headless",
            "--disable-gpu",
            f"--window-size={viewport_w},{viewport_h}",
            f"--screenshot={screenshot}",
            html.as_uri(),
        ]
        completed = subprocess.run(command, capture_output=True, text=True)
        if completed.returncode != 0 or not screenshot.exists():
            stderr = completed.stderr.strip() or completed.stdout.strip() or "unknown browser error"
            raise SystemExit(f"failed to rasterize SVG: {stderr}")
        return Image.open(screenshot).convert("RGBA")


def find_browser() -> str | None:
    for candidate in SVG_BROWSERS:
        if Path(candidate).exists():
            return candidate
    for candidate in ("msedge", "chrome", "chromium", "chromium-browser"):
        resolved = shutil.which(candidate)
        if resolved:
            return resolved
    return None


def fit_image(image: Image.Image, max_width: int, max_height: int) -> Image.Image:
    image = image.copy()
    image.thumbnail((max_width, max_height), Image.Resampling.LANCZOS)
    if image.width == 0 or image.height == 0:
        raise SystemExit("image resized to zero pixels")
    return image


def quantize_to_vga(image: Image.Image) -> list[int]:
    pixels: list[int] = []
    rgba = image.load()
    for y in range(image.height):
        for x in range(image.width):
            r, g, b, a = rgba[x, y]
            if a < 32:
                pixels.append(0)
            else:
                pixels.append(nearest_vga_index(r, g, b))
    return pixels


def nearest_vga_index(r: int, g: int, b: int) -> int:
    best_index = 0
    best_distance = None
    for index, (pr, pg, pb) in enumerate(VGA_PALETTE):
        dr = r - pr
        dg = g - pg
        db = b - pb
        distance = dr * dr + dg * dg + db * db
        if best_distance is None or distance < best_distance:
            best_distance = distance
            best_index = index
    return best_index


def encode_timg(width: int, height: int, pixels: list[int]) -> bytes:
    header = bytearray(TIMG_SIGNATURE)
    header.extend(width.to_bytes(2, "little"))
    header.extend(height.to_bytes(2, "little"))

    body = bytearray()
    index = 0
    while index < len(pixels):
        high = pixels[index] & 0x0F
        low = pixels[index + 1] & 0x0F if index + 1 < len(pixels) else 0
        body.append((high << 4) | low)
        index += 2
    return bytes(header + body)


if __name__ == "__main__":
    raise SystemExit(main())
