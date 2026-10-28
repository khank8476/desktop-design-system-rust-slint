import os
from PIL import Image, ImageDraw

def create_app_icon():
    os.makedirs("assets", exist_ok=True)
    size = 256
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 1. Outer rounded container (Squircle)
    margin = 12
    # Draw dark modern gradient background
    for y in range(margin, size - margin):
        ratio = (y - margin) / (size - 2 * margin)
        # Deep navy (#0f172a) to rich royal blue (#1e3a8a)
        r = int(15 + ratio * (30 - 15))
        g = int(23 + ratio * (58 - 23))
        b = int(42 + ratio * (138 - 42))
        draw.line(
            [(margin + 16, y), (size - margin - 16, y)],
            fill=(r, g, b, 255),
            width=1
        )

    # Clean rounded corner squircle mask
    mask = Image.new("L", (size, size), 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.rounded_rectangle(
        [margin, margin, size - margin, size - margin],
        radius=52,
        fill=255
    )

    # Apply rounded mask
    app_bg = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    app_bg.paste(img, (0, 0), mask=mask)
    draw = ImageDraw.Draw(app_bg)

    # 2. Add subtle modern border ring
    draw.rounded_rectangle(
        [margin, margin, size - margin, size - margin],
        radius=52,
        outline=(59, 130, 246, 180), # Electric blue border
        width=4
    )

    # 3. Draw Design System Component Geometry (3 Stacked Perspective Layer Cards)
    # Layer 1 (Bottom layer - Accent Blue)
    draw.polygon(
        [(128, 168), (196, 134), (128, 100), (60, 134)],
        fill=(37, 99, 235, 230),
        outline=(96, 165, 250, 255)
    )

    # Layer 2 (Middle layer - Vibrant Cyan)
    draw.polygon(
        [(128, 140), (196, 106), (128, 72), (60, 106)],
        fill=(14, 165, 233, 240),
        outline=(125, 211, 252, 255)
    )

    # Layer 3 (Top layer - Pure White / Pearl)
    draw.polygon(
        [(128, 112), (196, 78), (128, 44), (60, 78)],
        fill=(255, 255, 255, 250),
        outline=(224, 242, 254, 255)
    )

    # Save as PNG
    png_path = "assets/app-icon.png"
    app_bg.save(png_path, "PNG")
    print(f"Saved PNG to {png_path}")

    # Save as multi-resolution ICO for Windows .exe
    ico_path = "assets/app-icon.ico"
    app_bg.save(
        ico_path,
        format="ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    )
    print(f"Saved multi-res ICO to {ico_path}")

if __name__ == "__main__":
    create_app_icon()

