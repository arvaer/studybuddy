import { describe, expect, it } from "vitest";
import { MIN_STAY_SECONDS, ReadingClock } from "./reading";

describe("ReadingClock", () => {
  it("turns leaving a page into a stay, and drops flips", () => {
    const clock = new ReadingClock();
    expect(clock.enter("r", 1, 0)).toBeNull();
    // A flip through page 1 after two seconds does not count.
    expect(clock.enter("r", 2, 2000)).toBeNull();
    // Ninety seconds on page 2 does.
    expect(clock.enter("r", 3, 92_000)).toEqual({ resourceId: "r", page: 2, seconds: 90 });
    // Leaving the reader closes the last stay; leaving again is nothing.
    expect(clock.leave(100_000)).toEqual({ resourceId: "r", page: 3, seconds: 8 });
    expect(clock.leave(200_000)).toBeNull();
  });

  it("caps a stay at an hour and rounds to whole seconds", () => {
    const clock = new ReadingClock();
    clock.enter("r", 5, 0);
    expect(clock.leave(5 * 3600 * 1000)).toEqual({ resourceId: "r", page: 5, seconds: 3600 });
    clock.enter("r", 6, 0);
    expect(clock.leave(MIN_STAY_SECONDS * 1000 + 400)?.seconds).toBe(MIN_STAY_SECONDS);
  });
});
