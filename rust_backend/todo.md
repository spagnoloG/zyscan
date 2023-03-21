# TODO

## Python script
- [x] Replace argument `image_file` with `images_dir` to scan whole directory instead of single image. Just so we don't for each image spawn a subprocess in rust.
- [x] Print the image file path and the milvus id back to the rust code

## Rust part
- [x] Parse all the files and put them into the mongodb (create a new serializable struct)
- [x] Extract milvus id, let the mongoid be the same as id of mivus one
