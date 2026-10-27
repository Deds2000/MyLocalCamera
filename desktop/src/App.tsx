import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

function App() {
  const { t } = useTranslation();
  const [dialogOpen, setDialogOpen] = useState(false);

  return (
    <main className="flex h-full flex-col items-center justify-center gap-8 p-8">
      <h1 className="text-3xl font-semibold tracking-tight">{t("desktop.smoke.title")}</h1>

      <Tabs defaultValue="a" className="w-full max-w-md">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="a">{t("desktop.smoke.tabA")}</TabsTrigger>
          <TabsTrigger value="b">{t("desktop.smoke.tabB")}</TabsTrigger>
        </TabsList>
        <TabsContent value="a" className="flex justify-center">
          <Button onClick={() => setDialogOpen(true)}>{t("desktop.smoke.open")}</Button>
        </TabsContent>
        <TabsContent value="b" className="flex justify-center">
          <Button variant="outline" onClick={() => setDialogOpen(true)}>
            {t("desktop.smoke.open")}
          </Button>
        </TabsContent>
      </Tabs>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("desktop.smoke.dialogTitle")}</DialogTitle>
          </DialogHeader>
        </DialogContent>
      </Dialog>
    </main>
  );
}

export default App;
