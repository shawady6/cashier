import { useState } from "react";
import PageHeader from "../../components/PageHeader";
import Tabs from "../../components/Tabs";
import ExpensesTab from "./ExpensesTab";
import SessionTab from "./SessionTab";

const tabs = [
  { id: "session", label: "وردية الخزينة" },
  { id: "expenses", label: "المصروفات" },
] as const;

export default function TreasuryPage() {
  const [tab, setTab] = useState<(typeof tabs)[number]["id"]>("session");
  return (
    <div className="max-w-5xl">
      <PageHeader title="المصروفات والخزينة" description="فتح وإغلاق الوردية بجرد النقدية، حركات الدرج اليدوية، وتسجيل المصروفات." />
      <Tabs tabs={tabs} value={tab} onChange={setTab} />
      {tab === "session" ? <SessionTab /> : <ExpensesTab />}
    </div>
  );
}
