# Artifacts

The artifact store landed with issue #11 (migration `20240106000000_artifacts.sql`). Failure and backup behaviour is #12.

## What an artifact is

One upload. An immutable row in `artifacts` (`user_id`, `sha256`, `size_bytes`, `content_type`, `original_filename`, `uploaded_at`) and the bytes at `<UPLOADS_DIR>/artifacts/<sha256[0..2]>/<sha256>`. The address is the SHA-256 of the bytes, so:

- **A same-name upload cannot replace an earlier one.** It is a new artifact at a new address. The old `<uploads>/<user>/<filename>` layout, where a second `notes.pdf` overwrote the first, is gone for new uploads.
- **Identical bytes share one blob** but are two artifacts with their own metadata. The blob is written once and never rewritten.
- **A blob write is atomic.** Bytes go to a temporary file in the same directory and are renamed into place, so a reader never sees a partial blob. If a concurrent writer of the same bytes wins the rename, the loser discards its temporary file and proceeds.
- **Reads verify the address.** `ArtifactStore::read` hashes the bytes it serves and refuses a blob that no longer matches its row.

## How the records link

```
artifacts ──< resources.artifact_id            (an uploaded resource points at its upload)
artifacts ──< activity_revisions.source_artifact_id   (a revision pins the version it cited)
```

`source_artifact_id` is copied from the resource **at authoring time** by the adapter; the client never supplies it. Re-uploading or re-pointing the resource afterwards does not change what an earlier revision cites. The foreign key is `NO ACTION`, so an artifact a revision cites cannot be deleted. Gate step 7 (open the cited source version after a later same-name upload) is `a_revision_pins_the_source_version_it_was_authored_against` in `backend/crates/infra/tests/artifacts.rs`.

Resources created before this migration keep `file_path` only; their `artifact_id` is null. Nothing migrates them automatically (the plan authorises no changes to existing uploaded documents).

## API

| Route | Does |
| --- | --- |
| `POST /api/resources/upload` | unchanged shape; now stores an artifact first and returns `artifactId` on the resource |
| `GET /api/artifacts/{id}` | metadata, owner-scoped |
| `GET /api/artifacts/{id}/bytes` | the exact bytes with `Content-Type`, `Content-Disposition: inline` and an `ETag` of the address |

`GET /api/resources/{id}/file` still works and reads the same blob through `file_path`.

## Ownership

`artifacts.user_id` is in every predicate. A foreign artifact is `NotFound` on both routes. Uploading records the caller as owner; there is no way to attach an artifact to another learner's resource because the resource insert carries the caller's `user_id` and the artifact was just created for the same caller.

## Not yet defined (#12)

What happens when the blob write succeeds and the row insert fails (today: an orphan blob with no row, harmless), when the row exists but the blob is missing (today: `read` fails with an unexpected error), orphan detection, and backup and restore of the artifact directory alongside the database.
