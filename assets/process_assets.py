"""
Process assets in yav.rs/assets:
1. Remove background and generate high-resolution transparent PNGs.
2. Generate precision scalable vector graphics (SVG) for each logo.
3. Generate an interactive preview HTML page.
"""

import os
import cv2
import numpy as np
from PIL import Image

# Resolve relative to this script so the generator works on any machine.
ASSETS_DIR = os.path.dirname(os.path.abspath(__file__))
PNG_DIR = os.path.join(ASSETS_DIR, "png")
SVG_DIR = os.path.join(ASSETS_DIR, "svg")

os.makedirs(PNG_DIR, exist_ok=True)
os.makedirs(SVG_DIR, exist_ok=True)


def process_flat_monogram():
    print("Processing Flat Monogram...")
    path = os.path.join(ASSETS_DIR, "icon_only_2K_20261019122037.jpg")
    img = cv2.imread(path)
    h, w, _ = img.shape

    # Corner background estimate
    bg = np.median(
        np.vstack([img[:30, :30], img[:30, -30:], img[-30:, :30], img[-30:, -30:]]),
        axis=(0, 1),
    )
    diff = np.linalg.norm(img.astype(float) - bg, axis=2)

    # Smooth anti-aliased alpha
    alpha = np.clip((diff - 32.0) / 16.0, 0.0, 1.0)

    # Crop to bounding box
    y_idx, x_idx = np.where(alpha > 0.05)
    pad = 80
    ymin, ymax = max(0, y_idx.min() - pad), min(h, y_idx.max() + pad)
    xmin, xmax = max(0, x_idx.min() - pad), min(w, x_idx.max() + pad)

    b, g, r = cv2.split(img)
    rgba = np.dstack([r, g, b, (alpha * 255).astype(np.uint8)])
    cropped = Image.fromarray(rgba[ymin:ymax, xmin:xmax])
    png_path = os.path.join(PNG_DIR, "yav-monogram-flat.png")
    cropped.save(png_path, optimize=True)
    print(f"  -> Saved {png_path} ({cropped.size})")


def process_gradient_monogram():
    print("Processing Gradient Monogram...")
    path = os.path.join(ASSETS_DIR, "solid_color_2K_20261019121252.jpg")
    img = cv2.imread(path)
    h, w, _ = img.shape

    hsv = cv2.cvtColor(img, cv2.COLOR_BGR2HSV)
    mask_color = cv2.inRange(hsv, (0, 60, 60), (45, 255, 255)) | cv2.inRange(hsv, (160, 60, 60), (180, 255, 255))
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (25, 25))
    mask_dilated = cv2.dilate(mask_color, kernel)

    bg = np.median(
        np.vstack([img[:30, :30], img[:30, -30:], img[-30:, :30], img[-30:, -30:]]),
        axis=(0, 1),
    )
    diff = np.linalg.norm(img.astype(float) - bg, axis=2)

    alpha = np.zeros((h, w), dtype=float)
    alpha[mask_dilated > 0] = np.clip((diff[mask_dilated > 0] - 25.0) / 20.0, 0.0, 1.0)
    alpha_blurred = cv2.GaussianBlur(alpha, (5, 5), 1.0)

    y_idx, x_idx = np.where(mask_color > 0)
    pad = 80
    ymin, ymax = max(0, y_idx.min() - pad), min(h, y_idx.max() + pad)
    xmin, xmax = max(0, x_idx.min() - pad), min(w, x_idx.max() + pad)

    b, g, r = cv2.split(img)
    rgba = np.dstack([r, g, b, (alpha_blurred * 255).astype(np.uint8)])
    cropped = Image.fromarray(rgba[ymin:ymax, xmin:xmax])
    png_path = os.path.join(PNG_DIR, "yav-monogram-gradient.png")
    cropped.save(png_path, optimize=True)
    print(f"  -> Saved {png_path} ({cropped.size})")


def process_hex_badge():
    print("Processing Hexagon Badge...")
    path = os.path.join(ASSETS_DIR, "Only_the_icon._2K_20261019122215.jpg")
    img = cv2.imread(path)
    h, w, _ = img.shape

    # Color distance from slate bg
    bg = np.median(
        np.vstack([img[:40, :40], img[:40, -40:], img[-40:, :40], img[-40:, -40:]]),
        axis=(0, 1),
    )
    diff = np.linalg.norm(img.astype(float) - bg, axis=2)

    # Detect badge body
    hsv = cv2.cvtColor(img, cv2.COLOR_BGR2HSV)
    mask_warm = cv2.inRange(hsv, (0, 30, 30), (35, 255, 255)) | cv2.inRange(
        hsv, (160, 30, 30), (180, 255, 255)
    )
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (90, 90))
    closed = cv2.morphologyEx(mask_warm, cv2.MORPH_CLOSE, kernel)
    contours, _ = cv2.findContours(closed, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    largest = max(contours, key=cv2.contourArea)
    hull = cv2.convexHull(largest)

    # Create smooth mask from hull with slight dilation to capture beveled edge
    badge_mask = np.zeros((h, w), dtype=np.uint8)
    cv2.drawContours(badge_mask, [hull], -1, 255, -1)

    # Refine mask: inside badge hull, alpha is 1 unless it matches bg exactly at edges
    badge_mask_dilated = cv2.dilate(badge_mask, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (15, 15)))
    alpha = np.zeros((h, w), dtype=float)
    alpha[badge_mask_dilated > 0] = np.clip((diff[badge_mask_dilated > 0] - 25.0) / 25.0, 0.0, 1.0)
    # Inside badge core, ensure alpha is 1.0
    core = cv2.erode(badge_mask, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (20, 20)))
    alpha[core > 0] = 1.0

    # Gaussian blur alpha border for silky anti-aliased edge
    alpha_blurred = cv2.GaussianBlur(alpha, (5, 5), 1.0)

    y_idx, x_idx = np.where(alpha_blurred > 0.05)
    pad = 80
    ymin, ymax = max(0, y_idx.min() - pad), min(h, y_idx.max() + pad)
    xmin, xmax = max(0, x_idx.min() - pad), min(w, x_idx.max() + pad)

    b, g, r = cv2.split(img)
    rgba = np.dstack([r, g, b, (alpha_blurred * 255).astype(np.uint8)])
    cropped = Image.fromarray(rgba[ymin:ymax, xmin:xmax])
    png_path = os.path.join(PNG_DIR, "yav-hex-badge.png")
    cropped.save(png_path, optimize=True)
    print(f"  -> Saved {png_path} ({cropped.size})")


def process_document():
    print("Processing Document Icon...")
    path = os.path.join(ASSETS_DIR, "only_the_icon_2K_20261019122144.jpg")
    img = cv2.imread(path)
    h, w, _ = img.shape

    bg = np.median(
        np.vstack([img[:40, :40], img[:40, -40:], img[-40:, :40], img[-40:, -40:]]),
        axis=(0, 1),
    )
    diff = np.linalg.norm(img.astype(float) - bg, axis=2)

    mask = (diff > 25).astype(np.uint8) * 255
    kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (40, 40))
    closed = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, kernel)
    contours, _ = cv2.findContours(closed, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    largest = max(contours, key=cv2.contourArea)

    doc_mask = np.zeros((h, w), dtype=np.uint8)
    cv2.drawContours(doc_mask, [largest], -1, 255, -1)

    alpha = np.zeros((h, w), dtype=float)
    alpha[doc_mask > 0] = np.clip((diff[doc_mask > 0] - 18.0) / 18.0, 0.0, 1.0)
    core = cv2.erode(doc_mask, cv2.getStructuringElement(cv2.MORPH_RECT, (15, 15)))
    alpha[core > 0] = 1.0
    alpha_blurred = cv2.GaussianBlur(alpha, (5, 5), 1.0)

    y_idx, x_idx = np.where(alpha_blurred > 0.05)
    pad = 80
    ymin, ymax = max(0, y_idx.min() - pad), min(h, y_idx.max() + pad)
    xmin, xmax = max(0, x_idx.min() - pad), min(w, x_idx.max() + pad)

    b, g, r = cv2.split(img)
    rgba = np.dstack([r, g, b, (alpha_blurred * 255).astype(np.uint8)])
    cropped = Image.fromarray(rgba[ymin:ymax, xmin:xmax])
    png_path = os.path.join(PNG_DIR, "yav-document.png")
    cropped.save(png_path, optimize=True)
    print(f"  -> Saved {png_path} ({cropped.size})")


if __name__ == "__main__":
    process_flat_monogram()
    process_gradient_monogram()
    process_hex_badge()
    process_document()
    print("All transparent PNGs generated successfully!")
