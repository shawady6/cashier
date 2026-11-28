import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { api, type AppFolders, type BackupCheck, type BackupInfo } from "../../services/api";
import { useSettingsStore } from "../../store/settingsStore";

const KIND_LABEL: Record<BackupInfo["kind"], string> = {
  manual: "يدوية",
  auto: "تلقائية",
  "pre-migration": "قبل التحديث",
  "pre-restore": "قبل الاستعادة",
  "pre-update": "قبل التحديث",
};

const keyOf = (b: BackupInfo) => `${b.location}/${b.fileName}`;

function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024)).toLocaleString("ar-EG")} ك.ب`;
  return `${(bytes / (1024 * 1024)).toLocaleString("ar-EG", { maximumFractionDigits: 1 })} م.ب`;
}

export default function BackupSection() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [list, setList] = useState<BackupInfo[]>([]);
  const [checks, setChecks] = useState<Record<string, BackupCheck>>({});
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [restoring, setRestoring] = useState<BackupInfo | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [restarting, setRestarting] = useState(false);
  const [keep, setKeep] = useState("14");
  const [folders, setFolders] = useState<AppFolders | null>(null);

  const refresh = useCallback(async () => {
    try {
      setList(await api.listBackups());
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh, settings.backup_external_dir]);

  useEffect(() => {
    setKeep(settings.backup_keep_count ?? "14");
  }, [settings.backup_keep_count]);

  useEffect(() => {
    api.appFolders().then(setFolders).catch(() => undefined);
  }, []);

  async function run<T>(fn: () => Promise<T>): Promise<T | undefined> {
    setBusy(true);
    setMsg(null);
    try {
      return await fn();
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
      return undefined;
    } finally {
      setBusy(false);
    }
  }

  async function createNow() {
    const r = await run(() => api.createBackup());
    if (!r) return;
    const parts = [`تم إنشاء النسخة ${r.backup.fileName} وفحصها وهي سليمة`];
    if (r.externalCopied) parts.push("وتم نسخها للمجلد الخارجي");
    setMsg({ ok: !r.externalError, text: r.externalError ? `${parts[0]}، لكن تعذر النسخ للمجلد الخارجي: ${r.externalError}` : parts.join(" ") });
    await refresh();
  }

  async function check(b: BackupInfo) {
    const c = await run(() => api.checkBackup(b.location, b.fileName));
    if (c) setChecks((m) => ({ ...m, [keyOf(b)]: c }));
  }

  async function remove(b: BackupInfo) {
    setDeleting(null);
    const done = await run(async () => {
      await api.deleteBackup(b.location, b.fileName);
      return true;
    });
    if (done) {
      setMsg({ ok: true, text: "تم حذف النسخة" });
      await refresh();
    }
  }

  async function restore(b: BackupInfo) {
    const safety = await run(() => api.restoreBackup(b.location, b.fileName));
    setRestoring(null);
    if (safety) {
      setRestarting(true);
      setMsg({ ok: true, text: `تم تجهيز الاستعادة وحفظ نسخة أمان (${safety.fileName}). يُعاد تشغيل البرنامج الآن` });
    }
  }

  async function chooseFolder() {
    const picked = await run(() => api.pickFolder());
    if (!picked) return;
    const ok = await run(async () => {
      await update("backup_external_dir", picked);
      return true;
    });
    if (ok) setMsg({ ok: true, text: "تم تغيير مكان النسخة الإضافية" });
  }

  async function useDefaultFolder() {
    const ok = await run(async () => {
      await update("backup_external_dir", "");
      return true;
    });
    if (ok) setMsg({ ok: true, text: "تمت العودة للمجلد الافتراضي" });
  }

  async function openFolder() {
    await run(() => api.openAppFolder("root"));
  }

  async function saveOptions() {
    const n = Number(keep);
    if (!Number.isInteger(n) || n < 3 || n > 365) {
      setMsg({ ok: false, text: "عدد النسخ المحتفظ بها بين 3 و 365" });
      return;
    }
    const ok = await run(async () => {
      await update("backup_keep_count", String(n));
      return true;
    });
    if (ok) setMsg({ ok: true, text: "تم حفظ إعدادات النسخ الاحتياطي" });
  }

  return (
    <>
      {msg && <Alert kind={msg.ok ? "ok" : "error"}>{msg.text}</Alert>}

      <div className="flex flex-wrap items-center gap-3">
        <button type="button" className={btnPrimary} disabled={busy || restarting} onClick={() => void createNow()}>
          إنشاء نسخة احتياطية الآن
        </button>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={settings.backup_auto_enabled !== "0"}
            onChange={(e) => void run(() => update("backup_auto_enabled", e.target.checked ? "1" : "0"))}
          />
          نسخة تلقائية يوميًا
        </label>
      </div>

      <div className="space-y-3 rounded-lg border border-slate-200 p-3 dark:border-slate-700">
        <div className="text-sm">
          <div className="text-xs text-slate-500 dark:text-slate-400">مكان النسخة الإضافية</div>
          <div dir="ltr" className="break-all text-start font-medium">{settings.backup_external_dir || folders?.backups || "—"}</div>
        </div>
        <div className="flex flex-wrap gap-2">
          <button type="button" className={btnPrimary} disabled={busy} onClick={() => void chooseFolder()}>
            اختيار مجلد…
          </button>
          {settings.backup_external_dir && (
            <button type="button" className={btnGhost} disabled={busy} onClick={() => void useDefaultFolder()}>
              المجلد الافتراضي
            </button>
          )}
          <button type="button" className={btnGhost} disabled={busy} onClick={() => void openFolder()}>
            فتح مجلد البرنامج
          </button>
        </div>
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <label className="block text-sm">
          عدد النسخ التلقائية المحفوظة
          <input className={`${inputClass} mt-1 w-32`} inputMode="numeric" dir="ltr" value={keep} onChange={(e) => setKeep(e.target.value)} />
        </label>
        <button type="button" className={btnGhost} disabled={busy} onClick={() => void saveOptions()}>
          حفظ
        </button>
      </div>

      {list.length === 0 ? (
        <p className="text-sm text-slate-500 dark:text-slate-400">لا توجد نسخ احتياطية بعد</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-200 text-start text-xs text-slate-500 dark:border-slate-700 dark:text-slate-400">
                <th className="py-2 text-start font-medium">التاريخ</th>
                <th className="text-start font-medium">النوع</th>
                <th className="text-start font-medium">المكان</th>
                <th className="text-start font-medium">الحجم</th>
                <th className="text-start font-medium">الفحص</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {list.map((b) => {
                const k = keyOf(b);
                const c = checks[k];
                return (
                  <tr key={k} className="border-b border-slate-100 dark:border-slate-700/60">
                    <td className="py-2">{new Date(b.modifiedAt * 1000).toLocaleString("ar-EG")}</td>
                    <td>{KIND_LABEL[b.kind]}</td>
                    <td>{b.location === "local" ? "داخل البرنامج" : "خارجي"}</td>
                    <td>{formatSize(b.sizeBytes)}</td>
                    <td className={c ? (c.ok ? "text-teal-700" : "text-red-700") : "text-slate-400"}>{c ? c.message : "—"}</td>
                    <td className="space-x-2 whitespace-nowrap py-2 text-end space-x-reverse">
                      {deleting === k ? (
                        <>
                          <button type="button" className="text-red-700 hover:underline" disabled={busy} onClick={() => void remove(b)}>
                            تأكيد الحذف
                          </button>
                          <button type="button" className="hover:underline" onClick={() => setDeleting(null)}>
                            إلغاء
                          </button>
                        </>
                      ) : (
                        <>
                          <button type="button" className="text-teal-700 hover:underline" disabled={busy} onClick={() => void check(b)}>
                            فحص
                          </button>
                          <button type="button" className="text-teal-700 hover:underline" disabled={busy || restarting} onClick={() => setRestoring(b)}>
                            استعادة
                          </button>
                          <button type="button" className="text-red-700 hover:underline" disabled={busy} onClick={() => setDeleting(k)}>
                            حذف
                          </button>
                        </>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {restoring && (
        <Modal title="استعادة نسخة احتياطية" onClose={() => setRestoring(null)}>
          <div className="space-y-4 text-sm">
            <p>
              ستُستبدل بيانات البرنامج الحالية بنسخة {new Date(restoring.modifiedAt * 1000).toLocaleString("ar-EG")}، وكل ما سُجّل بعدها لن يظهر. تُحفظ نسخة أمان من البيانات الحالية قبل الاستبدال، ثم يُعاد تشغيل البرنامج.
            </p>
            <div className="flex gap-3">
              <button type="button" className={btnPrimary} disabled={busy} onClick={() => void restore(restoring)}>
                استعادة وإعادة التشغيل
              </button>
              <button type="button" className={btnGhost} onClick={() => setRestoring(null)}>
                إلغاء
              </button>
            </div>
          </div>
        </Modal>
      )}
    </>
  );
}
