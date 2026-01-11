import { createBrowserRouter } from "react-router";
import Layout from "./Layout";
import ErrorBoundary from "./ErrorBoundary";
import TranslateTab from "../features/translate/TranslateTab";
import EnhanceTab from "../features/enhance/EnhanceTab";
import TypeGenTab from "../features/typegen/TypeGenTab";
import GeneralTab from "../features/settings/GeneralTab";
import HistoryTab from "../features/history/HistoryTab";
import ShortcutsTab from "../features/shortcuts/ShortcutsTab";
import TutorialTab from "../features/tutorial/TutorialTab";
import FeedbackTab from "../features/feedback/FeedbackTab";
import TokenTab from "../features/token/TokenTab";
import SessionManagerTab from "../features/qa/SessionManagerTab";
import SessionHistoryTab from "../features/qa/SessionHistoryTab";
import SessionDetailPage from "../features/qa/SessionDetailPage";

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
        path: "tutorial",
        element: <TutorialTab />,
      },
    ],
  },
]);
