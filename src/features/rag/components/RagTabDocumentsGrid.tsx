import { FileText, Trash2 } from "lucide-react";
import type { RagDocument } from "../types";
import { getFileIcon } from "../ragTabUtils";
import AnimatedContainer from "../../../shared/components/AnimatedContainer";

type Props = {
  documents: RagDocument[];
  onDeleteDocument: (id: number) => void;
};

export function RagTabDocumentsGrid({ documents, onDeleteDocument }: Props) {
  if (documents.length === 0) {
    return (
      <AnimatedContainer animation="fadeIn">
        <div className="text-center py-20">
          <div className="w-24 h-24 mx-auto mb-6 rounded-full from-app-accent/5 to-app-accent/10 border border-app-border/50 flex items-center justify-center">
            <FileText className="w-12 h-12 text-app-text-muted/70" />
          </div>
          <h3 className="text-2xl font-bold text-app-text mb-3">No Documents Yet</h3>
          <p className="text-app-text-muted max-w-md mx-auto text-base leading-relaxed">
            Your collection is ready to store documents. Drag & drop files above to import them.
          </p>
        </div>
      </AnimatedContainer>
    );
  }

  return (
    <>
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-app-text">Documents</h2>
          <p className="text-sm text-app-text-muted mt-1">
            {documents.length} {documents.length === 1 ? "document" : "documents"} stored in this collection
          </p>
        </div>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {documents.map((document) => (
          <div
            key={document.id}
            className="group relative from-app-card to-app-card/80 rounded-2xl border border-app-border/50 p-6 hover:shadow-2xl hover:border-app-accent/30 hover:-translate-y-1 transition-all duration-300">
            <div className="absolute top-4 right-4">
              <div className="p-2 rounded-full bg-app-bg/80 backdrop-blur-sm border border-app-border/30 group-hover:border-app-accent/50 transition-colors">
                {getFileIcon(document.file_type)}
              </div>
            </div>

            <div className="mb-4 pr-10">
              <h3 className="font-semibold text-app-text truncate pr-8" title={document.file_name}>
                {document.file_name}
              </h3>
              <div className="mt-2 flex items-center justify-between gap-2">
                <span className="inline-flex items-center px-2.5 py-1 rounded-md bg-app-accent/10 text-app-accent text-xs font-medium">
                  {document.file_type.toUpperCase()}
                </span>
                <button
                  onClick={() => void onDeleteDocument(document.id)}
                  className="text-app-text-muted hover:text-red-500 transition-colors"
                  title="Delete document">
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>

            <div className="space-y-2.5">
              <div className="flex items-center justify-between text-sm">
                <span className="text-app-text-muted/70">Pages</span>
                <span className="font-medium text-app-text">{document.total_pages}</span>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="text-app-text-muted/70">Language</span>
                <span className="font-medium text-app-text capitalize">
                  {document.language === "auto" ? "Auto" : document.language}
                </span>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="text-app-text-muted/70">Added</span>
                <span className="font-medium text-app-text">
                  {new Date(document.created_at).toLocaleDateString(undefined, {
                    month: "short",
                    day: "numeric",
                    year: "numeric",
                  })}
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
