import { useEffect, useState } from "react";
import type { AnswerLanguage } from "../ragChatUtils";

export function useRagPersistedSettings() {
  const [selectedCollectionId, setSelectedCollectionId] = useState<number | null>(
    null,
  );
  const [answerLanguage, setAnswerLanguage] = useState<AnswerLanguage>("id");
  const [strictRagMode, setStrictRagMode] = useState(false);
  const [topK, setTopK] = useState(5);
  const [candidateK, setCandidateK] = useState(100);
  const [rerankK, setRerankK] = useState(75);
  const [dbFinalK, setDbFinalK] = useState(10);

  useEffect(() => {
    const savedCollection = localStorage.getItem("rag-selected-collection");
    if (savedCollection) {
      setSelectedCollectionId(parseInt(savedCollection, 10));
    }
    const savedLanguage = localStorage.getItem("rag-answer-language");
    if (savedLanguage === "en" || savedLanguage === "id") {
      setAnswerLanguage(savedLanguage);
    }

    const savedStrict = localStorage.getItem("rag-strict-mode");
    if (savedStrict === "1" || savedStrict === "true") {
      setStrictRagMode(true);
    }

    const savedTopK = localStorage.getItem("rag-top-k");
    if (savedTopK) {
      const v = parseInt(savedTopK, 10);
      if (!Number.isNaN(v) && v > 0) setTopK(v);
    }

    const savedCandidateK = localStorage.getItem("rag-candidate-k");
    if (savedCandidateK) {
      const v = parseInt(savedCandidateK, 10);
      if (!Number.isNaN(v) && v > 0) setCandidateK(v);
    }

    const savedRerankK = localStorage.getItem("rag-rerank-k");
    if (savedRerankK) {
      const v = parseInt(savedRerankK, 10);
      if (!Number.isNaN(v) && v > 0) setRerankK(v);
    }

    const savedDbFinalK = localStorage.getItem("rag-db-final-k");
    if (savedDbFinalK) {
      const v = parseInt(savedDbFinalK, 10);
      if (!Number.isNaN(v) && v > 0) setDbFinalK(v);
    }
  }, []);

  useEffect(() => {
    if (selectedCollectionId) {
      localStorage.setItem(
        "rag-selected-collection",
        selectedCollectionId.toString(),
      );
    }
  }, [selectedCollectionId]);

  useEffect(() => {
    localStorage.setItem("rag-answer-language", answerLanguage);
  }, [answerLanguage]);

  useEffect(() => {
    localStorage.setItem("rag-strict-mode", strictRagMode ? "1" : "0");
  }, [strictRagMode]);

  useEffect(() => {
    localStorage.setItem("rag-top-k", String(topK));
  }, [topK]);

  useEffect(() => {
    localStorage.setItem("rag-candidate-k", String(candidateK));
  }, [candidateK]);

  useEffect(() => {
    localStorage.setItem("rag-rerank-k", String(rerankK));
  }, [rerankK]);

  useEffect(() => {
    localStorage.setItem("rag-db-final-k", String(dbFinalK));
  }, [dbFinalK]);

  return {
    selectedCollectionId,
    setSelectedCollectionId,
    answerLanguage,
    setAnswerLanguage,
    strictRagMode,
    setStrictRagMode,
    topK,
    setTopK,
    candidateK,
    setCandidateK,
    rerankK,
    setRerankK,
    dbFinalK,
    setDbFinalK,
  };
}
