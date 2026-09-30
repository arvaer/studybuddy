# Artifacts

The artifact store landed with issue #11 (migration `20240106000000_artifacts.sql`); failure, audit and backup behaviour with #12.

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

## Failure behaviour (#12)

Every upload does three writes in a fixed order: the blob, then the `artifacts` row, then the `resources` row. Because the blob comes first, the catalog and the directory can only disagree in one direction, and each case has a defined outcome. All five are tests in `backend/crates/infra/tests/artifact_failures.rs`.

| Case | What is left | What the learner sees | What the audit says |
| --- | --- | --- | --- |
| Blob written, row insert fails | a complete blob at its address, no row | the upload request fails (500); nothing to retry against, so they upload again | `orphan <address>` until they do; the retry reuses the blob and the audit is clean |
| Row exists, blob missing (disk restored from an older backup, file removed by hand) | the row and every resource or revision that cites it | metadata (`GET /api/artifacts/{id}`) still answers; bytes fail with 500 and the log names the address, never 404, because the catalog says it exists | `missing <address>`; restoring the exact bytes at the address heals it with no catalog change |
| Partial upload (client body ends early) | nothing: the store is not touched until the whole multipart field is in memory | 422 `incomplete upload` | clean |
| Write interrupted before the rename (process killed mid-write) | a `.<uuid>.part` file next to the blobs, never a half-written blob at an address | nothing; a `.part` is never served and never mistaken for a blob | `part <path>` |
| Concurrent uploads of the same bytes | one blob, one row each | each request succeeds | clean |

A row is never written before its bytes are durable, so a restart at any point cannot produce a row that points at nothing. `ArtifactStore::read` refuses a missing blob (`blob missing at <address>`) and an altered one (`bytes do not match their address`) with distinct messages; both are 500 at the API because the catalog is authoritative and the failure is the operator's to fix.

The artifact row and the resource row are not one transaction. An artifact whose resource insert failed is a valid artifact with no resource pointing at it, which the API cannot reach except through `GET /api/artifacts/{id}`. That is acceptable: it is an upload the learner made, and it is not an orphan in the storage sense.

## Orphan detection

```sh
cd backend && cargo run -- audit-artifacts
```

Reads `DATABASE_URL` and `UPLOADS_DIR` like the server, does not migrate, and prints one line per finding (`missing`, `orphan`, `part`) and a summary. It exits `1` if any blob is missing and `0` otherwise. It never deletes: the plan authorises no deletion of uploaded documents, and an orphan blob costs only disk. Removing orphans or stray parts is a deliberate operator action taken from the report; a `.part` older than the longest plausible upload is safe to remove, an orphan blob is safe to remove once you are sure no backup restore is pending that would add a row for it.

Deleting a user cascades to their artifact rows but not to blobs (other users' artifacts may share them), so a user deletion also produces orphans. That is expected.

## Backup and restore

**What.** Two things, and only two: the database (`pg_dump` of the whole database) and the uploads directory (`UPLOADS_DIR`, default `backend/data/uploads`). The directory holds the content-addressed blobs under `artifacts/` and, for resources uploaded before #11, the legacy `<user>/<filename>` files that `resources.file_path` still points at. Back up the whole directory.

**Order.** Dump the database first, then copy the directory. An upload that lands between the two steps is an orphan blob in the backup, which is harmless; the reverse order could leave the backup with a row whose blob was never copied, which is data loss. Blobs are never modified after their rename, so copying a live directory is safe and an incremental copy (`rsync -a`) only transfers new addresses.

```sh
pg_dump "$DATABASE_URL" --format=custom --file=studybuddy.dump
rsync -a --exclude='artifacts/*/.*.part' "$UPLOADS_DIR"/ backup/uploads/
```

**Restore.** Restore the database dump, put the directory back at `UPLOADS_DIR`, then run the audit. `0 missing` is the success condition. `missing` lines name addresses to recover from an older copy of the directory; an address is the SHA-256 of the file, so any copy from anywhere is the right bytes if its hash matches, and `read` will verify that on every request anyway.

```sh
pg_restore --dbname "$DATABASE_URL" studybuddy.dump
rsync -a backup/uploads/ "$UPLOADS_DIR"/
cd backend && cargo run -- audit-artifacts
```

**What is not backed up.** Nothing else holds learner data. Extracted text and pages live in `resources`, so they come with the dump. There is no cache to rebuild.
