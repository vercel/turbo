import "../styles.css";
import "../custom.css";

import { SSRProvider } from "@react-aria/ssr";
import type { AppProps } from "next/app";
import { type ReactNode } from "react";
import { Analytics } from "@vercel/analytics/react";
import { VercelToolbar } from "@vercel/toolbar/next";
import { useRouter } from "next/router";
import { getCommentsState } from "../lib/comments";

type NextraAppProps = AppProps & {
  Component: AppProps["Component"] & {
    getLayout: (page: ReactNode) => ReactNode;
  };
};

// Shim requestIdleCallback in Safari
if (typeof window !== "undefined" && !("requestIdleCallback" in window)) {
  // @ts-expect-error -- window isn't typed
  // eslint-disable-next-line @typescript-eslint/no-implied-eval, @typescript-eslint/no-unsafe-argument  -- Not sure what this code is so let's play it safe and leave it here.
  window.requestIdleCallback = (fn) => setTimeout(fn, 1);
  // @ts-expect-error -- window isn't typed
  window.cancelIdleCallback = (e) => {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-argument -- Not sure what this code is so let's play it safe and leave it here.
    clearTimeout(e);
  };
}

const toolbarEnabledPaths = ["/repo/docs", "/pack/docs"];

export default function Nextra({ Component, pageProps }: NextraAppProps) {
  const router = useRouter();

  const pathHasToolbar = toolbarEnabledPaths.some((path) =>
    router.asPath.startsWith(path)
  );

  return (
    <SSRProvider>
      <>
        {/**
         * Globally defined svg linear gradient, for use in icons
         */}
        <svg height="0px" width="0px">
          <defs>
            <linearGradient
              id="pink-gradient"
              x1="0%"
              x2="100%"
              y1="0%"
              y2="100%"
            >
              <stop offset="0%" stopColor="rgba(156, 81, 161, 1)" />
              <stop offset="70%" stopColor="rgba(255, 30, 86, 1)" />
            </linearGradient>
          </defs>
        </svg>
      </>
      <Component {...pageProps} />
      <Analytics />
      {getCommentsState() && pathHasToolbar ? <VercelToolbar /> : null}
    </SSRProvider>
  );
}
