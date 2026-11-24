export default function Alert({ kind, children }: { kind: "error" | "ok"; children: React.ReactNode }) {
  return (
    <div
      role={kind === "error" ? "alert" : "status"}
      className={`rounded-lg p-3 text-sm ${kind === "ok" ? "bg-teal-50 text-teal-800" : "bg-red-50 text-red-700"}`}
    >
      {children}
    </div>
  );
}
