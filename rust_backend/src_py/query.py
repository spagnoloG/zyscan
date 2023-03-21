""" QUERY SCRIPT """
from argparse import ArgumentParser
import clip
import torch
from pymilvus import (
        connections,
        FieldSchema,
        CollectionSchema,
        DataType,
        Collection,
)

_METRIC_TYPE = 'L2'
_INDEX_TYPE = 'IVF_FLAT'
_NLIST = 1024
_NPROBE = 16
_TOPK = 3
_VECTOR_FIELD = "image_embedding"

def db_connect() -> None:
    try:
        connections.connect("default", host="localhost", port="19530")
    except Exception as e:
        print(e)
        exit(1)

def embed_query(args: dict) -> torch.Tensor:
    """ Process input string """
    try:
        MODEL, PREPROCESS = clip.load("ViT-B/32", device=args.device)
    except Exception as e:
        raise ValueError(f"Failed to load model: {e}")

    try:
        text = clip.tokenize(args.user_query).to(args.device)
    except Exception as e:
        raise ValueError(f"Failed to tokenize text: {e}")

    try:
        text_features = MODEL.encode_text(text)
    except Exception as e:
        raise ValueError(f"Failed to encode text: {e}")

    return text_features.detach().cpu().numpy()


def search_db(text_features: torch.Tensor, args: dict) -> list:
    """ Search database for similar images """
    z_images_collection = Collection(name="z_images")

    ### CREATE INDEX ###
    try:
        create_index(z_images_collection, _VECTOR_FIELD)
    except Exception as e:
        raise ValueError(f"Failed to create index: {e}")
    ###

    ### LOAD COLLECTION ###
    try:
        z_images_collection.load()
    except Exception as e:
        raise ValueError(f"Failed to load collection: {e}")

    # Search in the milvus db for the similar vectors
    try:
        search_param = {
                "data": [text_features],
                "anns_field": _VECTOR_FIELD,
                "param": {"metric_type": _METRIC_TYPE, "params": {"nprobe": _NPROBE}},
                "limit": _TOPK,
                "expr": "id_field >= 0"
        }

        results = z_images_collection.search(
                **search_param
        )
        for i, result in enumerate(results):
            print("\nSearch result for {}th vector: ".format(i))
            for j, res in enumerate(result):
                print("Top {}: {}".format(j, res))

    except Exception as e:
        raise ValueError(f"Failed to search database: {e}")

    print(results)


def create_index(collection, filed_name):
    index_param = {
        "index_type": _INDEX_TYPE,
        "params": {"nlist": _NLIST},
        "metric_type": _METRIC_TYPE}
    collection.create_index(filed_name, index_param)
    print("\nCreated index:\n{}".format(collection.index().params))

def main():
    """ Main function """
    parser = ArgumentParser()
    parser.add_argument(
        '--user_query',
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
    if args.device not in ["cpu", "cuda"]:
        raise ValueError(f"Invalid device: {args.device}")
    if args.device == "cuda" and not torch.cuda.is_available():
        raise ValueError("CUDA is not available on this device.")

    db_connect()
    embeded_query = embed_query(args)
    search_db(embeded_query, args)


if __name__ == "__main__":
    main()
