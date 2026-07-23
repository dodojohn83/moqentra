import { Route, Routes } from "react-router-dom";
import { Annotations } from "../pages/Annotations";
import { Applications } from "../pages/Applications";
import { Datasets } from "../pages/Datasets";
import { Deployments } from "../pages/Deployments";
import { Home } from "../pages/Home";
import { Login } from "../pages/Login";
import { Models } from "../pages/Models";
import { NotFound } from "../pages/NotFound";
import { OidcCallback } from "../pages/OidcCallback";
import { Projects } from "../pages/Projects";
import { Training } from "../pages/Training";
import { ProtectedRoute } from "./ProtectedRoute";

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/oidc/callback" element={<OidcCallback />} />
      <Route
        path="/"
        element={
          <ProtectedRoute>
            <Home />
          </ProtectedRoute>
        }
      />
      <Route
        path="/projects"
        element={
          <ProtectedRoute>
            <Projects />
          </ProtectedRoute>
        }
      />
      <Route
        path="/datasets"
        element={
          <ProtectedRoute>
            <Datasets />
          </ProtectedRoute>
        }
      />
      <Route
        path="/annotations"
        element={
          <ProtectedRoute>
            <Annotations />
          </ProtectedRoute>
        }
      />
      <Route
        path="/training"
        element={
          <ProtectedRoute>
            <Training />
          </ProtectedRoute>
        }
      />
      <Route
        path="/models"
        element={
          <ProtectedRoute>
            <Models />
          </ProtectedRoute>
        }
      />
      <Route
        path="/applications"
        element={
          <ProtectedRoute>
            <Applications />
          </ProtectedRoute>
        }
      />
      <Route
        path="/deployments"
        element={
          <ProtectedRoute>
            <Deployments />
          </ProtectedRoute>
        }
      />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
