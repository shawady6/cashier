import { useState } from "react";
import PageHeader from "../../components/PageHeader";
import Tabs from "../../components/Tabs";
import BalancesTab from "./BalancesTab";
import CountsTab from "./CountsTab";
import MovementsTab from "./MovementsTab";
import TransfersTab from "./TransfersTab";
import WarehousesTab from "./WarehousesTab";

const tabs = [
  { id: "balances", label: "الأرصدة" },
  { id: "movements", label: "حركة المخزون" },
  { id: "transfers", label: "التحويلات" },
  { id: "counts", label: "الجرد" },
  { id: "warehouses", label: "المخازن" },
] as const;

export default function InventoryPage() {
  const [tab, setTab] = useState<(typeof tabs)[number]["id"]>("balances");
  return (
    <div className="max-w-6xl">
      <PageHeader title="المخزون" description="الأرصدة بمتوسط التكلفة، الحركات، التحويلات بين المخازن، والجرد." />
      <Tabs tabs={tabs} value={tab} onChange={setTab} />
      {tab === "balances" && <BalancesTab />}
      {tab === "movements" && <MovementsTab />}
      {tab === "transfers" && <TransfersTab />}
      {tab === "counts" && <CountsTab />}
      {tab === "warehouses" && <WarehousesTab />}
    </div>
  );
}
