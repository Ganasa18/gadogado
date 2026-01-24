import { createBrowserRouter } from "react-router";
import Layout from "./Layout";
import ErrorBoundary from "./ErrorBoundary";
import TranslateTab from "../features/translate";
import EnhanceTab from "../features/enhance";
import TypeGenTab from "../features/typegen";
import GeneralTab from "../features/settings";
import HistoryTab from "../features/history";
import ShortcutsTab from "../features/shortcuts";
import TutorialTab from "../features/tutorial";
import FeedbackTab from "../features/feedback";
import TokenTab from "../features/token";
import RagTab from "../features/rag/RagTab";
import RagChat from "../features/rag/RagChat";
import RagAnalytics from "../features/rag/RagAnalytics";
import VisualizeJsonPage from "../features/visualize-json/VisualizeJsonPage";
import { DiffCheckerPage } from "../features/diff-checker";
import MockServerTab from "../features/mock-server";
import {
  AiResultsPage,
  SessionDetailPage,
  SessionHistoryTab,
  SessionManagerTab,
} from "../features/qa";
import MenuManagementTab from "../features/settings/pages/MenuManagementTab";
import DatabaseTab from "../features/settings/pages/DatabaseTab";
import {
  SetupTab,
  TrainTab,
  EvaluateTab,
  ExportTab,
} from "../features/model-distillation";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <Layout />,
    errorElement: <ErrorBoundary />,
    children: [
      {
        index: true,
        element: <GeneralTab />,
      },
      {
        path: "translate",
        element: <TranslateTab />,
      },
      {
        path: "enhance",
        element: <EnhanceTab />,
      },
      {
        path: "typegen",
        element: <TypeGenTab />,
      },
      {
        path: "mock-server",
        element: <MockServerTab />,
      },
      {
        path: "history",
        element: <HistoryTab />,
      },
      {
        path: "qa",
        element: <SessionManagerTab />,
      },
      {
        path: "qa/history",
        element: <SessionHistoryTab />,
      },
      {
        path: "qa/session/:id",
        element: <SessionDetailPage />,
      },
      {
        path: "qa/session/:id/ai",
        element: <AiResultsPage />,
      },
      {
        path: "rag",
        element: <RagTab />,
      },
      {
        path: "rag-chat",
        element: <RagChat />,
      },
      {
        path: "rag/analytics",
        element: <RagAnalytics />,
      },
      {
        path: "token",
        element: <TokenTab />,
      },
      {
        path: "general",
        element: <GeneralTab />,
      },
      {
        path: "shortcut",
        element: <ShortcutsTab />,
      },
      {
        path: "feedback",
        element: <FeedbackTab />,
      },
      {
        path: "menu-management",
        element: <MenuManagementTab />,
      },
      {
        path: "database",
        element: <DatabaseTab />,
      },
      {
        path: "visualize-json",
        element: <VisualizeJsonPage />,
      },
      {
        path: "diff-checker",
        element: <DiffCheckerPage />,
      },
      {
        path: "tutorial",
        element: <TutorialTab />,
      },
      {
        path: "model-destilation/setup",
        element: <SetupTab />,
      },
      {
        path: "model-destilation/train",
        element: <TrainTab />,
      },
      {
        path: "model-destilation/evaluate",
        element: <EvaluateTab />,
      },
      {
        path: "model-destilation/export",
        element: <ExportTab />,
      },
    ],
  },
]);
