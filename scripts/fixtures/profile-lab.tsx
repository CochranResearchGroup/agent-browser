import React from "react";
import { createRoot } from "react-dom/client";
import { ProfileLab } from "@/components/profile-lab";
createRoot(document.getElementById("root")!).render(<ProfileLab profiles={[{ id: "donor", name: "Synthetic donor" }]} onRefresh={async () => {}} />);
