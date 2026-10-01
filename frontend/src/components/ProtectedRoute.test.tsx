import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ProtectedRoute } from "./ProtectedRoute";

const auth = vi.hoisted(() => ({ user: null as null | { id: string }, isLoading: false }));
vi.mock("@/contexts/AuthContext", () => ({ useAuth: () => auth }));

function mount() {
  return render(
    <MemoryRouter initialEntries={["/quiz"]}>
      <Routes>
        <Route path="/auth" element={<p>login form</p>} />
        <Route path="/quiz" element={<ProtectedRoute><p>quiz page</p></ProtectedRoute>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("ProtectedRoute", () => {
  it("renders nothing while the session is still being restored, and does not redirect", () => {
    auth.user = null; auth.isLoading = true;
    mount();
    expect(screen.queryByText("quiz page")).toBeNull();
    expect(screen.queryByText("login form")).toBeNull();
  });

  it("redirects to /auth once restore finished with no user", () => {
    auth.user = null; auth.isLoading = false;
    mount();
    expect(screen.getByText("login form")).toBeTruthy();
  });

  it("renders the page for a signed-in user", () => {
    auth.user = { id: "u1" }; auth.isLoading = false;
    mount();
    expect(screen.getByText("quiz page")).toBeTruthy();
  });
});
