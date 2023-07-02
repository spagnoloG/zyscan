# Zyscan

This project is an image search application that utilizes the CLIP (Contrastive Language-Image Pretraining) model from OpenAI and the Milvus database to enable efficient searching through a large collection of images based on their content.

Features:
- Content-based Image Search: The application allows users to search for images based on the content they contain, rather than relying on metadata or textual descriptions.
- CLIP Model Integration: The CLIP model, developed by OpenAI, is used to generate image embeddings (vectors) that represent the visual content of each image. These embeddings are then stored in the Milvus database for fast similarity search.
- Efficient Image Indexing: The Milvus database is employed for efficient storage and indexing of the image embeddings. It provides low-latency and high-throughput similarity search capabilities, enabling quick retrieval of visually similar images.

