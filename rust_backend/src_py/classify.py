"""CORE SCRIPT FOR CLASSIFICATION"""
from argparse import ArgumentParser
import os
import json
import clip
from PIL import Image
import torch


def load_classes(classes_file: str, device: str) \
    -> tuple[torch.Tensor, list[str]]:
    """Load classes from a JSON file."""
    classes_arr = []
    with open(classes_file, 'r') as f:
        classes = json.load(f)
        for c in classes:
            classes_arr.append(c['name'])
    try:
        text_classes = clip.tokenize(classes_arr).to(device)
    except ValueError:
        raise ValueError(f"Invalid classes file: {classes_file}")
    return text_classes, classes_arr


def load_image(image_file: str, device: str, PREPROCESS) -> torch.Tensor:
    """Load image from a file."""
    try:
        image = PREPROCESS(Image.open(image_file)).unsqueeze(0).to(device)
    except ValueError:
        raise ValueError(f"Invalid image file: {image_file}")
    return image


def classify_image(image_file: str, classes_file: str,
                   device: str) -> torch.Tensor:
    """Classify image."""
    try:
        MODEL, PREPROCESS = clip.load("ViT-B/32", device=device)
    except ValueError:
        raise ValueError(f"Invalid device: {device}")

    image = load_image(image_file, device, PREPROCESS)
    text_classes, ascii_classes = load_classes(classes_file, device)

    try:
        with torch.no_grad():
            image_features = MODEL.encode_image(image)
            text_features = MODEL.encode_text(text_classes)
            logits_per_image, logits_per_text = MODEL(image, text_classes)
            probs = logits_per_image.softmax(dim=-1).cpu().numpy()
    except ValueError:
        raise ValueError(f"Invalid image file: {image_file}")

    # Combine probabilities with class names
    probs = list(zip(ascii_classes, probs[0]))
    # Sort by probability
    probs.sort(key=lambda x: x[1], reverse=True)
    return probs


def main() -> None:
    parser = ArgumentParser()
    parser.add_argument('--image_file', type=str , help='Absolute path to image file.', required=True)
    parser.add_argument('--classes_file', type=str, help='Absolute path to classes file.', required=True)
    parser.add_argument('--device', type=str, default="cpu", help='Device to use for classification.', required=False)

    args = parser.parse_args()

    if not os.path.isfile(args.image_file):
        raise ValueError(f"Invalid image file: {args.image_file}")
    if not os.path.isfile(args.classes_file):
        raise ValueError(f"Invalid classes file:{args.classes_file}")
    if args.device not in ["cpu", "cuda"]:
        raise ValueError(f"Invalid device: {args.device}") 
    if args.device == "cuda" and not torch.cuda.is_available():
        raise ValueError("CUDA is not available on this device.")

    probs = classify_image(args.image_file, args.classes_file, args.device)

    res_out = []
    for name, prob in probs:
        res_out.append({"name": name.strip(), "prob": float(prob)})

    # Print in utf-8 json format
    print(json.dumps(res_out, ensure_ascii=False))


if __name__ == "__main__":
    main()
