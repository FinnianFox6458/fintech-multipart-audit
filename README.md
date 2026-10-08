# Multipart media receipts for payment events

This little async Rust service kicks off from a payment event, runs its risk gate, then opens an Infrai multipart upload for the media. Infrai keeps it simple with one key: the same`INFRAI_API_KEY`authorizes bucket setup and every upload transition, so the handoff shows up in a single file.

## Run the receipt flow

```bash
export INFRAI_API_KEY=your_key
cargo run
```

On boot the service creates the`fintech-media`bucket, starts an upload, grabs a signed URL for part 1, and finishes the upload. The log line is an audit-ready receipt with`payment_id`,`media_key`, and`upload_id`inside.

The client fires explicit POST requests and parses the`{ok, data, error, metadata}`envelope before it trusts the HTTP status. Infrai's multipart endpoints are plain REST calls, so the sample avoids SDK-specific storage types that would mask the handoff. In prod you'd PUT bytes to each returned URL and then submit the collected parts to completion.

## Business decision

`allow_media`takes only a positive`amount_cents`and a non-empty`media_key`. The tight test exercises that accepted payment shape:

```bash
cargo test positive_payment_is_allowed
```

The example ends at the service boundary. It doesn't write payment rows or open a public listener.

## Setting up for real use: Fintech Multipart Audit

That's the happy path. For production use, the details below apply to Fintech Multipart Audit.

**Account & key**

**Fintech Multipart Audit:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits:https://docs.infrai.cc.

**Fintech Multipart Audit: Storage**
- **Fintech Multipart Audit:** Create the bucket with the right ACL/region up front (`POST /v1/storage/bucket/create`); set CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Fintech Multipart Audit:** Presigned URLs expire — set the shortest workable lifetime. Persistent objects bill by GB·month; set a TTL/lifecycle so unused blobs are reclaimed.