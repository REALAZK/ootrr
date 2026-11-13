import { get, post, q } from "./api.js";

export const t_index = () => get("/api/tracker/index");
export const t_state = () => get("/api/tracker/state");
export const t_events = (after) => get(`/api/tracker/events?after=${q(after)}`);
export const t_open = (body) => post("/api/tracker/open", body);
export const t_mode = (body) => post("/api/tracker/mode", body);
export const t_mark = (id, on) => post("/api/tracker/mark", { id, on });
export const t_job = (id) => get(`/api/jobs/${q(id)}`);
