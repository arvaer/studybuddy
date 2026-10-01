import { Navigate } from "react-router-dom";
import { useAuth } from "@/contexts/AuthContext";

/**
 * Guards a route behind a signed-in session. While the session is still being
 * restored from the cookie (`GET /api/auth/me` in flight) it renders nothing:
 * redirecting then would bounce every hard refresh of a protected page to the
 * login form even though the cookie is valid, which is what gate step 4
 * (refresh the page; the accepted activity and attempt remain) forbids.
 */
export function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, isLoading } = useAuth();
  if (isLoading) return null;
  if (!user) return <Navigate to="/auth" replace />;
  return <>{children}</>;
}
