"""CORE SCRIPT FOR CLASSIFICATION"""
from argparse import ArgumentParser
import os
import json
import clip
from PIL import Image
import torch
import numpy as np

from pymilvus import (
    connections,
    utility,
    FieldSchema, CollectionSchema, DataType,
    Collection,
)

IS_FIRST_RUN = True

def db_connect() -> None:
    try:
        connections.connect("default", host="localhost", port="19530")
    except Exception as e:
        print(e)
        exit(1)


def insert_image_into_db(db, image, image_vector) -> int:
    """Insert image into Milvus DB"""
    fields = [
        FieldSchema(
            name="pk",
            dtype=DataType.INT64,
            is_primary=True,
            auto_id=True),
        FieldSchema(
            name="image_embedding",
            dtype=DataType.FLOAT_VECTOR,
            dim=512),
    ]
    schema = CollectionSchema(
        fields, "Collection consisting of image embeddings")

    z_images_collection = Collection(
        "z_images", schema, consistency_level="Strong")

    status = z_images_collection.insert([image_vector])

    z_images_collection.flush()

    if status.succ_count == 1:
        return status.primary_keys[0]
    else:
        return 1


def load_classes(classes_file: str, device: str) \
        -> tuple[torch.Tensor, list[str]]:
    """Load classes from a JSON file."""
    classes_arr = []
    with open(classes_file, 'r') as f:
        classes = json.load(f)
        counter = 0
        for c in classes:
            counter += 1
            classes_arr.append(c['name'])

        print("Number of classes", counter)
    try:
        text_classes = clip.tokenize(classes_arr).to(device)
        print(text_classes.shape)
    except ValueError:
        raise ValueError(f"Invalid classes file: {classes_file}")
    return text_classes, classes_arr


def load_image(image_file: str, device: str, PREPROCESS) -> torch.Tensor:
    """Load image from a file."""
    try:
        img = Image.open(image_file)
        if img.mode != "RGB":
            img = img.convert("RGB")

        # Return empty tensor if image is too small
        if img.size[0] < 400 or img.size[1] < 400:
            return torch.empty(0)

        image = PREPROCESS(img).unsqueeze(0).to(device)
    except ValueError:
        raise ValueError(f"Invalid image file: {image_file}")
    return image


def euclidean_distance(x, y) -> float:
    """ Euclidean distance between two vectors """
    return np.sqrt(np.sum(np.square(x - y)))


def hellinger_distance(x, y) -> float:
    """ Hellinger distance between two vectors """
    x = x.reshape(-1)
    y = y.reshape(-1)
    return np.sqrt(0.5 * np.sum(np.square(np.sqrt(x) - np.sqrt(y))))


def chi_square_distance(x, y) -> float:
    """ Chi-square distance between two vectors """
    return 0.5 * np.sum(np.square(x - y) / (x + y + np.finfo(float).eps))


def intersection_distance(x, y) -> float:
    """ Intersection distance between two vectors """
    return (1 - np.sum(np.minimum(x, y)))


def log_processed_image(image_file: str, milvus_id: int) -> None:
    global IS_FIRST_RUN
    """Log processed image."""
    data = {
        "image_file": image_file,
        "milvus_id": milvus_id
    }
    str_json = json.dumps(data)
    if IS_FIRST_RUN:
        print(str_json, end="")
        IS_FIRST_RUN = False
    else:
        print("," + str_json, end="")


def classify_image(image_file: str, args: dict) -> bool:
    """Classify image."""
    try:
        MODEL, PREPROCESS = clip.load("ViT-B/32", device=args.device)
    except ValueError:
        raise ValueError(f"Invalid device: {args.device}")

    image = load_image(image_file, args.device, PREPROCESS)
    # The image did not meeet the minimum size requirements
    if image.shape[0] == 0:
        return True

    try:
        with torch.no_grad():
            image_features = MODEL.encode_image(image)

    except ValueError:
        raise ValueError(f"Invalid image file: {image_file}")

    id = insert_image_into_db(
        utility,
        image_file,
        image_features.cpu().numpy())
    if id == 1:
        return False

    log_processed_image(image_file, id)

    return True


def traverse_images(images_dir: str, args: dict) -> bool:
    """Traverse images directory."""
    traverse_images_recursively(images_dir, args)
    return True


def traverse_images_recursively(images_dir: str, args: dict):
    """Traverse images directory recursively."""

    for f in os.listdir(images_dir):
        if os.path.isdir(os.path.join(images_dir, f)):
            traverse_images_recursively(
                os.path.join(images_dir, f), args)
        # The file is an image, classify it
        else:
            if f.endswith(".jpg") or f.endswith(".png") or f.endswith(".jpeg"):
                if not classify_image(os.path.join(images_dir, f), args):
                    raise ValueError("Failed to classify image.")


def main() -> None:
    """Main function."""
    parser = ArgumentParser()
    parser.add_argument(
        '--images_dir',
        type=str,
        help='Absolute path to images directory..',
        required=True)
    parser.add_argument(
        '--device',
        type=str,
        default="cpu",
        help='Device to use for classification.',
        required=False)

    args = parser.parse_args()

    if not os.path.exists(args.images_dir):
        raise ValueError(f"Invalid images directory: {args.images_dir}")
    if args.device not in ["cpu", "cuda"]:
        raise ValueError(f"Invalid device: {args.device}")
    if args.device == "cuda" and not torch.cuda.is_available():
        raise ValueError("CUDA is not available on this device.")

    db_connect()

    # Just to format the output
    print("[", end="")
    if not traverse_images(args.images_dir, args):
        raise ValueError("something did not go to plan :(")
    else:
        print("]")
        return True


if __name__ == "__main__":
    main()
