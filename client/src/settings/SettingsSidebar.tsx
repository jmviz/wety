import { etyModeGroups } from "./etyModes";

import {
  Drawer,
  IconButton,
  Typography,
  List,
  ListItem,
  ListItemText,
  ListSubheader,
  Switch,
  Box,
  Button,
} from "@mui/material";
import MenuIcon from "@mui/icons-material/Menu";
import { useState } from "react";

interface SettingsSidebarProps {
  disabledEtyModes: Set<string>;
  setDisabledEtyModes: (modes: Set<string>) => void;
}

export default function SettingsSidebar({
  disabledEtyModes,
  setDisabledEtyModes,
}: SettingsSidebarProps) {
  const [open, setOpen] = useState(false);

  const toggleMode = (mode: string) => {
    const next = new Set(disabledEtyModes);
    if (next.has(mode)) {
      next.delete(mode);
    } else {
      next.add(mode);
    }
    setDisabledEtyModes(next);
  };

  const toggleGroup = (modes: string[]) => {
    const allDisabled = modes.every((m) => disabledEtyModes.has(m));
    const next = new Set(disabledEtyModes);
    for (const m of modes) {
      if (allDisabled) {
        next.delete(m);
      } else {
        next.add(m);
      }
    }
    setDisabledEtyModes(next);
  };

  return (
    <>
      <IconButton
        onClick={() => setOpen(true)}
        sx={{ position: "fixed", top: 8, left: 8, zIndex: 1200 }}
        aria-label="settings"
      >
        <MenuIcon />
      </IconButton>
      <Drawer anchor="left" open={open} onClose={() => setOpen(false)}>
        <Box sx={{ width: 280, pt: 2, pb: 2 }}>
          <Typography variant="h6" sx={{ px: 2, pb: 1 }}>
            Settings
          </Typography>
          <Typography variant="subtitle2" sx={{ px: 2, pb: 1, color: "text.secondary" }}>
            Connection types
          </Typography>
          <List dense disablePadding>
            {etyModeGroups.map((group) => {
              const allDisabled = group.modes.every((m) =>
                disabledEtyModes.has(m)
              );
              const someDisabled = group.modes.some((m) =>
                disabledEtyModes.has(m)
              );
              return (
                <Box key={group.label}>
                  <ListSubheader
                    sx={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      lineHeight: "36px",
                    }}
                  >
                    {group.label}
                    <Button
                      size="small"
                      onClick={() => toggleGroup(group.modes)}
                      sx={{ minWidth: 0, textTransform: "none", fontSize: "0.75rem" }}
                    >
                      {allDisabled ? "enable all" : someDisabled ? "enable all" : "disable all"}
                    </Button>
                  </ListSubheader>
                  {group.modes.map((mode) => (
                    <ListItem key={mode} sx={{ py: 0 }}>
                      <ListItemText
                        primary={mode}
                        primaryTypographyProps={{ variant: "body2" }}
                      />
                      <Switch
                        edge="end"
                        size="small"
                        checked={!disabledEtyModes.has(mode)}
                        onChange={() => toggleMode(mode)}
                      />
                    </ListItem>
                  ))}
                </Box>
              );
            })}
          </List>
        </Box>
      </Drawer>
    </>
  );
}
