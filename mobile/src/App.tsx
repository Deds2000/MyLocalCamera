import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CountdownRing } from "@/components/CountdownRing";
import { BigToggle } from "@/components/BigToggle";
import { LiveRing } from "@/components/LiveRing";
import { StatTile } from "@/components/StatTile";
import { SegmentedControl } from "@/components/SegmentedControl";
import { StatusChip } from "@/components/StatusChip";
import { Camera, Mic } from "lucide-react";

function App() {
  const { t } = useTranslation();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [seg, setSeg] = useState("a");

  return (
    <main className="flex h-full flex-col items-center justify-center gap-8 p-8">
      <h1 className="text-3xl font-semibold tracking-tight">{t("mobile.smoke.title")}</h1>

      <Tabs defaultValue="a" className="w-full max-w-md">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="a">{t("mobile.smoke.tabA")}</TabsTrigger>
          <TabsTrigger value="b">{t("mobile.smoke.tabB")}</TabsTrigger>
        </TabsList>
        <TabsContent value="a" className="flex justify-center">
          <Button onClick={() => setDialogOpen(true)}>{t("mobile.smoke.open")}</Button>
        </TabsContent>
        <TabsContent value="b" className="flex justify-center">
          <Button variant="outline" onClick={() => setDialogOpen(true)}>
            {t("mobile.smoke.open")}
          </Button>
        </TabsContent>
      </Tabs>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("mobile.smoke.dialogTitle")}</DialogTitle>
          </DialogHeader>
        </DialogContent>
      </Dialog>

      <section className="flex w-full max-w-md flex-col gap-4 rounded-lg border border-glass-border bg-glass p-4">
        <CountdownRing seconds={15} total={30} ariaLabel="Countdown" />
        <div className="flex gap-2">
          <BigToggle icon={<Camera className="h-5 w-5" />} label="Camera" pressed={false} />
          <BigToggle icon={<Mic className="h-5 w-5" />} label="Mic" pressed={true} />
        </div>
        <LiveRing active label="Live" />
        <div className="grid grid-cols-2 gap-2">
          <StatTile label="Bitrate" value="1.2" unit="Mbps" />
          <StatTile label="RTT" value={42} unit="ms" hint="p95" />
        </div>
        <SegmentedControl
          options={[
            { value: "a", label: "A" },
            { value: "b", label: "B" },
            { value: "c", label: "C", disabled: true, disabledReason: "Indisponivel" },
          ]}
          value={seg}
          onChange={setSeg}
          ariaLabel="Segmented"
        />
        <div className="flex gap-2">
          <StatusChip state="idle" label="Idle" />
          <StatusChip state="connecting" label="Connecting" />
          <StatusChip state="pending" label="Pending" />
          <StatusChip state="streaming" label="Streaming" />
          <StatusChip state="reconnecting" label="Reconnecting" />
          <StatusChip state="error" label="Error" />
        </div>
      </section>
    </main>
  );
}

export default App;
