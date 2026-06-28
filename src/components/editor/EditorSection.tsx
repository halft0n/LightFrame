import { useState, type ReactNode } from "react";

interface EditorSectionProps {
  title: string;
  defaultOpen?: boolean;
  children: ReactNode;
}

export function EditorSection({ title, defaultOpen = true, children }: EditorSectionProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <section className="border-b border-white/10">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between px-4 py-3 text-left text-sm font-medium text-neutral-200 transition hover:bg-white/5"
      >
        <span>{title}</span>
        <span
          className={`text-neutral-500 transition-transform duration-200 ${open ? "rotate-180" : ""}`}
        >
          ▾
        </span>
      </button>
      <div
        className={`overflow-hidden transition-all duration-200 ease-out ${
          open ? "max-h-[2000px] opacity-100" : "max-h-0 opacity-0"
        }`}
      >
        <div className="space-y-1 px-4 pb-4">{children}</div>
      </div>
    </section>
  );
}
