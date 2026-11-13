let token = "";

export class ApiError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

export function init_token() {
  const m = /[#&]token=([A-Za-z0-9]+)/.exec(location.hash);
  if (m) {
    token = m[1];
    try {
      sessionStorage.setItem("ootrr.token", token);
    } catch {
      token = m[1];
    }
    history.replaceState(null, "", location.pathname + location.search);
  } else {
    try {
      token = sessionStorage.getItem("ootrr.token") || "";
    } catch {
      token = "";
    }
  }
  return token.length > 0;
}

export async function api(method, path, body) {
  const heads = { "X-OOTRR-Token": token };
  const opts = { method, headers: heads, cache: "no-store", credentials: "omit", referrerPolicy: "no-referrer" };
  if (body !== undefined) {
    heads["Content-Type"] = "application/json";
    opts.body = JSON.stringify(body);
  }
  let res;
  try {
    res = await fetch(path, opts);
  } catch {
    throw new ApiError(0, "The local server did not answer. Is ootrr serve still running?");
  }
  let data = null;
  try {
    data = await res.json();
  } catch {
    data = null;
  }
  if (!res.ok) {
    const msg = res.status === 401
      ? "This page has no valid session token. Open the link that ootrr serve printed."
      : (data && data.error) || `The server answered ${res.status}.`;
    throw new ApiError(res.status, msg);
  }
  return data;
}

export const get = (path) => api("GET", path);
export const post = (path, body) => api("POST", path, body);
export const q = (v) => encodeURIComponent(v);

export async function get_blob(path) {
  let res;
  try {
    res = await fetch(path, { headers: { "X-OOTRR-Token": token }, cache: "no-store", credentials: "omit", referrerPolicy: "no-referrer" });
  } catch {
    throw new ApiError(0, "The local server did not answer. Is ootrr serve still running?");
  }
  if (!res.ok) throw new ApiError(res.status, `The server answered ${res.status}.`);
  return res.blob();
}
