import { describe, expect, it, vi } from "vitest";
import { ApiError } from "./api";
import type { AttemptReceipt } from "./activities";
import { memoryStorage, restore, saveDraft, submit, type AttemptClient } from "./attempt-state";

const REV = "rev-1";

function receipt(status: AttemptReceipt["status"] = "correct"): AttemptReceipt {
  return { attemptId: "att-1", activityRevisionId: REV, submittedAt: "2026-09-30T00:00:00Z", status, response: "B", assessment: null };
}

function client(over: Partial<AttemptClient> = {}): AttemptClient {
  let n = 0;
  return {
    record: vi.fn(async () => ({ receipt: receipt(), replayed: false })),
    fetch: vi.fn(async () => receipt()),
    newKey: () => `key-${++n}`,
    ...over,
  };
}

describe("submit", () => {
  it("records with a fresh key, keeps the attempt id and drops the draft", async () => {
    const storage = memoryStorage();
    const c = client();
    saveDraft(REV, storage, "B");

    const phase = await submit(REV, "B", storage, c);

    expect(phase).toEqual({ kind: "accepted", receipt: receipt(), replayed: false });
    expect(c.record).toHaveBeenCalledWith({ requestKey: "key-1", activityRevisionId: REV, response: "B", assistance: [] });
    expect(storage.load(REV)).toEqual({ attemptId: "att-1" });
  });

  it("retries a network failure with the same key, so the backend records once", async () => {
    const storage = memoryStorage();
    const record = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockResolvedValueOnce({ receipt: receipt(), replayed: true });
    const c = client({ record });

    const failed = await submit(REV, "B", storage, c);
    expect(failed).toEqual({ kind: "failed", message: "Failed to fetch", retryable: true });
    expect(storage.load(REV)).toEqual({ draft: "B", submission: { requestKey: "key-1", response: "B" } });

    const retried = await submit(REV, "B", storage, c);
    expect(retried).toEqual({ kind: "accepted", receipt: receipt(), replayed: true });
    expect(record.mock.calls.map((call) => call[0].requestKey)).toEqual(["key-1", "key-1"]);
  });

  it("mints a new key when the answer changed after a failure, never reusing one for a different payload", async () => {
    const storage = memoryStorage();
    const record = vi.fn().mockRejectedValueOnce(new ApiError(503, "down")).mockResolvedValueOnce({ receipt: receipt(), replayed: false });
    const c = client({ record });

    await submit(REV, "B", storage, c);
    await submit(REV, "C", storage, c);

    expect(record.mock.calls.map((call) => [call[0].requestKey, call[0].response])).toEqual([
      ["key-1", "B"],
      ["key-2", "C"],
    ]);
  });

  it("marks a 4xx as not retryable and keeps the draft", async () => {
    const storage = memoryStorage();
    const c = client({ record: vi.fn().mockRejectedValue(new ApiError(422, "response must be one of the options")) });

    const phase = await submit(REV, "Z", storage, c);

    expect(phase).toEqual({ kind: "failed", message: "response must be one of the options", retryable: false });
    expect(storage.load(REV).draft).toBe("Z");
    expect(storage.load(REV).attemptId).toBeUndefined();
  });
});

describe("restore", () => {
  it("re-reads a recorded attempt from the backend rather than trusting local state", async () => {
    const storage = memoryStorage();
    storage.save(REV, { attemptId: "att-1" });
    const c = client({ fetch: vi.fn(async () => receipt("pending")) });

    const phase = await restore(REV, storage, c);

    expect(c.fetch).toHaveBeenCalledWith("att-1");
    expect(phase).toEqual({ kind: "accepted", receipt: receipt("pending"), replayed: true });
  });

  it("forgets an attempt the backend no longer owns for this learner and falls back to the draft", async () => {
    const storage = memoryStorage();
    storage.save(REV, { attemptId: "att-other", draft: "B" });
    const c = client({ fetch: vi.fn().mockRejectedValue(new ApiError(404, "attempt att-other")) });

    expect(await restore(REV, storage, c)).toEqual({ kind: "draft" });
    expect(storage.load(REV)).toEqual({ draft: "B" });
  });

  it("is a draft when nothing was recorded", async () => {
    const storage = memoryStorage();
    saveDraft(REV, storage, "half an answer");
    const c = client();

    expect(await restore(REV, storage, c)).toEqual({ kind: "draft" });
    expect(c.fetch).not.toHaveBeenCalled();
    expect(storage.load(REV).draft).toBe("half an answer");
  });
});
