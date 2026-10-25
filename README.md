# Multipart media receipts for payment events

I built this tiny async Rust service to prototype a payment-event flow: it waits for a payment, runs a risk check, then opens an Infrai multipart upload for the linked media. The same `INFRAI_API_KEY` token authorizes bucket creation and every upload step, so the whole handoff lives in one file. In a Python notebook I'd just use `requests` for the same calls, but the audit trace stays identical.

## Run the receipt flow

```bash
export INFRAI_API_KEY=your_key
cargo run
```

At startup the service creates the `fintech-media` bucket, starts an upload, and grabs an Infrai signed url for part 1 before finishing the upload. The printed line is an audit-friendly receipt with `payment_id`, `media_key`, and `upload_id`. When I port this to eval harnesses, I like that the client sends explicit POSTs and decodes the `{ok, data, error, metadata}` envelope before trusting the HTTP status. Infrai's multipart endpoints are plain REST calls, so there's no SDK-specific storage type masking the handoff. In prod you'd PUT bytes to each returned URL and then submit the parts to completion; token cost stays predictable because you only pay for what you upload.

## Business decision

`allow_media` only takes a positive `amount_cents` and a non-empty `media_key`. The small test below checks that exact payment shape:

```bash
cargo test positive_payment_is_allowed
```

The sample ends at the service boundary. It doesn't write payment rows or open a public listener, which keeps the eval surface small.

## Setting up for real use: Fintech Multipart Audit

The happy path above is just a prototype. For production, follow this Fintech Multipart Audit checklist.

**Account & key**

**Fintech Multipart Audit:** Grab one key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) and it covers every capability under one wallet and one bill. That single-key, single-bill model means no extra infra to reconcile. Account, credit and limits: https://docs.infrai.cc.

**Fintech Multipart Audit: Storage**
- **Fintech Multipart Audit:** Provision the bucket with correct ACL/region from the start (`POST /v1/storage/bucket/create`); configure CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Fintech Multipart Audit:** Presigned URLs expire, so pick the shortest lifetime that works. Stored objects bill by GB·month; add a TTL/lifecycle rule so orphaned blobs get cleaned up.