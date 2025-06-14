import { parseArgs } from "jsr:@std/cli/parse-args";
import { deepMerge } from "jsr:@cross/deepmerge";
import { Cookie, getSetCookies } from "jsr:@std/http/cookie";

const USER_AGENT = "OSCPieVRChatAPIConnector/0.0.0 cathiecode@gmail.com";

function cookieHeader(cookies: Cookie[]): string {
  return cookies.map((c) => `${c.name}=${c.value}`).join("; ");
}

type VRChatAuthorization = {
  cookies: Cookie[];
};

class ErrorResponse {
  error: string;
  message: string;
  body: unknown;

  constructor(error: string, message: string, body: unknown) {
    this.error = error;
    this.message = message;
    this.body = body;
  }
}

class VRChatAPI {
  baseUrl: string;
  authorization: VRChatAuthorization;

  constructor(
    authorization: VRChatAuthorization,
    baseUrl: string = "https://api.vrchat.cloud/api/1",
  ) {
    this.baseUrl = baseUrl;
    this.authorization = authorization;
  }

  async request<T>(
    method: "GET" | "POST" | "PUT" | "DELETE",
    endpoint: string,
    body?: any,
    mergeRequestInit?: RequestInit,
  ) {
    const requestInit: RequestInit = {};

    const headers: HeadersInit = {};
    headers["User-Agent"] = USER_AGENT;

    if (body) {
      requestInit.body = JSON.stringify(body);
      headers["Content-Type"] = "application/json";
    }

    if (this.authorization) {
      headers["Cookie"] = cookieHeader(this.authorization.cookies);
    }

    requestInit.method = method;
    requestInit.headers = headers;

    const result = await fetch(
      `${this.baseUrl}${endpoint}`,
      deepMerge(requestInit, mergeRequestInit ?? {}),
    );

    if (!result.ok) {
      const errorBody = await result.json();
      throw new ErrorResponse(
        result.statusText,
        `Error ${result.status}: ${errorBody.message || "Unknown error"}`,
        errorBody,
      );
    }

    const cookies = getSetCookies(result.headers);

    cookies.forEach((cookie) => {
      const cookieWithSameName = this.authorization.cookies.find((c) =>
        c.name === cookie.name
      );

      if (cookieWithSameName) {
        // Update existing cookie
        Object.assign(cookieWithSameName, cookie);
      } else {
        // Add new cookie
        this.authorization.cookies.push(cookie);
      }
    });

    return await result.json() as T;
  }
}

type CurrentUserInfo = {
  id: string;
  displayName: string;
  requiresTwoFactorAuth: undefined;
} | {
  id: undefined;
  requiresTwoFactorAuth: ("totp" | "emailOtp")[];
};

type ConfigV1 = {
  version: "v1";
  userId: string | undefined;
  authorization: VRChatAuthorization;
};

type Config = ConfigV1;

const defaultConfig: Config = {
  version: "v1",
  userId: undefined,
  authorization: {
    cookies: [],
  },
};

function loadConfig(): Config {
  const config = localStorage.getItem("config");

  if (!config) {
    return defaultConfig;
  }

  const loadedConfig = JSON.parse(config) as Config;

  return loadedConfig;
}

function saveConfig(config: Config) {
  localStorage.setItem("config", JSON.stringify(config));
}

function editConfig(editFn: (config: Config) => void): Config {
  const config = loadConfig();
  editFn(config);
  saveConfig(config);

  return config;
}

async function login(
  api: VRChatAPI,
  args: Args,
) {
  const userName = args.username,
    password = args.password,
    twoFactorCode = args["tfa-code"],
    twoFactorMethod = args["tfa-method"];

  if (!userName || !password) {
    console.error("Username and password are required.");
    return;
  }

  if (twoFactorCode && !twoFactorMethod) {
    console.error("--tfa-code is required when providing a 2FA code.");
    return;
  }

  if (!["totp", "emailOtp", undefined].includes(twoFactorMethod)) {
    console.error(
      `Invalid 2FA method: ${twoFactorMethod}. Supported methods are: totp, emailOtp.`,
    );
    return;
  }

  let currentUser = await api.request<CurrentUserInfo>(
    "GET",
    "/auth/user",
    undefined,
    {
      headers: {
        "Authorization": `Basic ${
          btoa(`${encodeURI(userName)}:${encodeURI(password)}`)
        }`,
      },
    },
  );

  // 2FA
  const may2FAResponse = currentUser;

  if (may2FAResponse["requiresTwoFactorAuth"]) {
    const twoFactorMethods = may2FAResponse["requiresTwoFactorAuth"];

    if (
      twoFactorMethods.includes("totp") && twoFactorMethod === "totp" &&
      twoFactorCode
    ) {
      await api.request(
        "POST",
        "/auth/twofactorauth/totp/verify",
        { code: twoFactorCode },
      );

      currentUser = await api.request<CurrentUserInfo>("GET", "/auth/user");
    } else if (
      twoFactorMethods.includes("emailOtp") && twoFactorMethod === "emailOtp" &&
      twoFactorCode
    ) {
      await api.request(
        "POST",
        "/auth/twofactorauth/emailotp/verify",
        { code: twoFactorCode },
      );
      currentUser = await api.request<CurrentUserInfo>("GET", "/auth/user");
    } else {
      throw new Error(
        `API requires 2FA by one of the methods: ${
          twoFactorMethods.join(", ")
        }, but no valid code was provided.`,
      );
    }
  } else {
    // Something went wrong or vrchat changed their policy?
    console.error(
      "API did not require 2FA, but the response indicates it should have.",
    );
  }

  if (currentUser.id === undefined) {
    throw new Error("Login failed: Invalid username or password.");
  }

  editConfig((c) => {
    c.userId = currentUser.id;
  });

  console.log(`Logged in with ${currentUser.displayName}`);
}

async function changeStatus(api: VRChatAPI, args: Args) {
  const status = args["status"];
  const statusDescription = args["status-description"];

  if (
    !["active", "join me", "ask me", "busy", "offline", undefined].includes(
      status,
    )
  ) {
    throw new Error(
      `Invalid status: ${status}. Supported values are: "active", "join me", "ask me", "busy", "offline".`,
    );
  }

  const userId = loadConfig().userId;

  if (!userId) {
    throw new Error("User ID is not set. Please log in first.");
  }

  const body: Record<string, string> = {};
  if (status) {
    body.status = status;
  }
  if (statusDescription) {
    body.statusDescription = statusDescription;
  }

  await api.request("PUT", `/users/${userId}`, body);

  console.log("Status updated successfully.");
}

function clean() {
  localStorage.clear();
  console.log("Local storage cleared.");
}

type Args = {
  username?: string;
  password?: string;
  "tfa-code"?: string;
  "tfa-method"?: string;
  "status"?: string;
  "status-description"?: string;
};

async function main() {
  const args = parseArgs(Deno.args, {
    string: [
      "username",
      "password",
      "tfa-code",
      "tfa-method",
      "status",
      "status-description",
    ],
  });

  const command = args._[0];

  const config = loadConfig();

  const api = new VRChatAPI(config.authorization);

  try {
    switch (command) {
      case "login":
        await login(api, args);
        break;
      case "change_status":
        await changeStatus(api, args);
        break;
      case "clean":
        clean();
        break;
      case "info":
        console.log(JSON.stringify(config));
        break;
      default:
        console.error(
          `Unknown command: ${command}. Supported commands: login.`,
        );
        break;
    }
  } catch (error) {
    if (error instanceof Error) {
      console.error(`Error: ${error.message}`);
    } else {
      console.error(`An unknown error occurred: ${JSON.stringify(error)}`);
    }
  }

  editConfig((c) => {
    c.authorization = api.authorization;
  });
}

main();
