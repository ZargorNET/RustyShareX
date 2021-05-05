# Rusty ShareX

Another version of a ShareX custom, simple server, but now written in Rustlang!\
This results in raw performance, speed and really low idle-memory consume!

# Features

- Image upload (and GIF)
- Binary upload (like PDF)
- Custom IDs
- Fancy preview using JS (only images)
- Simple authentication (basically only a plain password, no account fiddling!)
- Delete via ShareX "Delete URL"
- (Maybe in future text upload with code highlighting)

# Compiling

`cargo build --release` (possibly nightly version needed)\
or just build the Dockerfile

# Running

Set a few environment variables and then just run the executable!\
A MongoDB database is required though.

| Name              | Description                                                                 |
|-------------------|-----------------------------------------------------------------------------|
| BIND              | The bind address like 0.0.0.0:8080                                          |
| UPLOAD_PW         | The password required to upload                                             |
| MONGO_URI         | Mongo Uri used to connect to the database                                   |
| MONGO_DB          | The Mongo database where the collection live                                |
| MONGO_HEADER_COLL | The header collection inside the database                                   |
| MONGO_CHUNK_COLL  | The chunks collection inside the database                                   |
| CHUNKS_SIZE       | This is optional! The chunk size in bytes, but just leave it as its default |

# License

MIT - do what you want