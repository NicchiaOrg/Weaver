# System Certificates

## system-certificates

- **Certificates kept:** a certificate goes in the bundle only when it parses as X.509 and its basic constraints say it is a certificate authority. Unreadable and non-CA certificates are skipped without failing. Identical certificates (same DER bytes) appear once, at their first place: the system's first, then the public roots.
- **No authority:** an export whose system sources give no certificate authority fails with `no certificate authority found`, even though the public roots alone would fill a bundle.
- **Age:** a bundle is stale when it is missing or its modification time is more than 24 h before now. At start, a stale bundle is exported again; while Weaver stays open, the next export is due 24 h after the last one ended.
- **Automatic exports** are those at start, after the Zscaler turn-on and the daily ones; an export asked with Refresh, or by turning the setting on in Settings, is not automatic.
- **Zscaler turn-on, once per user:** only when Zscaler is detected (`requirements/tech#system-certificates`), the setting is off, and `systemCertificates.autoEnabledAt` is absent. Afterwards Weaver never turns the setting on by itself again, even if the developer turns it off and Zscaler stays installed. With the setting already on, nothing is written or remembered.
- **Texts tested:** the whole stderr of a failed omp command, an ACP error message, and the stderr tail of a job or General; not only the line kept as the reason.
- **Certificate error:** the text contains one of
  - these codes, matching case: `UNABLE_TO_GET_ISSUER_CERT_LOCALLY`, `UNABLE_TO_GET_ISSUER_CERT`, `UNABLE_TO_VERIFY_LEAF_SIGNATURE`, `SELF_SIGNED_CERT_IN_CHAIN`, `DEPTH_ZERO_SELF_SIGNED_CERT`, `CERT_HAS_EXPIRED`, `CERT_UNTRUSTED`, `ERR_TLS_CERT_ALTNAME_INVALID`;
  - these phrases, ignoring case: `unable to get local issuer certificate`, `self signed certificate in certificate chain`, `self-signed certificate in certificate chain`, `certificate verify failed`, `SSL certificate problem`, `curl: (60)`.
- **HTTP error:** only when the text is not a certificate error. Codes: `403`, `407` and every code from `500` to `599`, as a whole number (not part of a longer number). Forms, ignoring case:
  - `HTTP <code>`, `HTTP/<version> <code>`;
  - `status <code>`, `status: <code>`, `status code <code>`, `statusCode: <code>`;
  - `returned error: <code>`;
  - the code followed by its standard reason phrase: `403 Forbidden`, `407 Proxy Authentication Required`, `500 Internal Server Error`, `501 Not Implemented`, `502 Bad Gateway`, `503 Service Unavailable`, `504 Gateway Timeout`, `505 HTTP Version Not Supported`, `506 Variant Also Negotiates`, `507 Insufficient Storage`, `508 Loop Detected`, `510 Not Extended`, `511 Network Authentication Required`.
  - The first match in the text gives the code. Other codes (`401`, `404`, `429`…), a code followed by words that are not its standard phrase, and bare numbers are not HTTP errors.
