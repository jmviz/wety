import { TreeKind } from "./types";

import Box from "@mui/material/Box";
import InputLabel from "@mui/material/InputLabel";
import MenuItem from "@mui/material/MenuItem";
import FormControl from "@mui/material/FormControl";
import Select, { SelectChangeEvent } from "@mui/material/Select";

interface TreeKindSelectProps {
  selectedTreeKind: TreeKind;
  setSelectedTreeKind: (treeMode: TreeKind) => void;
}

export default function TreeKindSelect({
  selectedTreeKind,
  setSelectedTreeKind,
}: TreeKindSelectProps) {
  const handleChange = (event: SelectChangeEvent) => {
    setSelectedTreeKind(event.target.value as TreeKind);
  };

  return (
    <Box sx={{ minWidth: 120 }}>
      <FormControl fullWidth>
        <InputLabel>Mode</InputLabel>
        <Select value={selectedTreeKind} label="Mode" onChange={handleChange}>
          {Object.values(TreeKind).map((kind) => (
            <MenuItem key={kind} value={kind}>
              {kind}
            </MenuItem>
          ))}
        </Select>
      </FormControl>
    </Box>
  );
}
