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
| ID_LENGTH         | This is optional! This will set how long the image id is                    |

# ShareX Configuration

Sample

```json
{
  "Version": "12.4.1",
  "Name": "Zargor.NET",
  "DestinationType": "ImageUploader",
  "RequestMethod": "POST",
  "RequestURL": "https://s.zrgr.pw/u",
  "Parameters": {
    "id": "$prompt:Custom ID?$"
  },
  "Headers": {
    "Authorization": "<Your password here>"
  },
  "Body": "Binary",
  "URL": "https://s.zrgr.pw/$json:id$",
  "ThumbnailURL": "https://s.zrgr.pw/$json:id$",
  "DeletionURL": "https://s.zrgr.pw/$json:id$/d/$json:deleteKey$"
}
```

# License

MIT - do what you want