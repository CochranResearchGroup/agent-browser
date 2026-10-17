// A private fixture configuration is not a copy of the operator's credentials.
// Keep explicit runtime inputs; never serialize the general parent environment.
const allowed=new Set([
 'HOME','PATH','LANG','LANGUAGE','LC_ALL','LC_CTYPE','TZ','USER','LOGNAME','SHELL',
 'TMPDIR','TMP','TEMP','LD_LIBRARY_PATH','FONTCONFIG_FILE','FONTCONFIG_PATH',
 'DISPLAY','XAUTHORITY','PULSE_SERVER','DBUS_SESSION_BUS_ADDRESS',
 'XDG_RUNTIME_DIR','XDG_CACHE_HOME','XDG_CONFIG_HOME','XDG_DATA_HOME',
 'REMOTE_VIEW_SLOT_GENERATION','AGENT_BROWSER_HOME','AGENT_BROWSER_SOCKET_DIR',
 'AGENT_BROWSER_RUNTIME_HOST','AGENT_BROWSER_SESSION_DISPLAY',
 'AGENT_BROWSER_EXECUTABLE_PATH','AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD',
 'AGENT_BROWSER_SITEGUIDE_DIRECTORY',
]);
export function browserFixtureEnvironment(environment){return Object.fromEntries(Object.entries(environment).filter(([key])=>allowed.has(key)));}
