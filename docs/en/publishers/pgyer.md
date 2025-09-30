# Pgyer

The pgyer target publishes your `.apk` or `.ipa` artifacts to the [pgyer.com](https://pgyer.com).

## Set up environment variables

requires some environment variables set up to run correctly.

```
export PGYER_API_KEY="your api key"
```

## Usage

Run:

```
fastforge publish \
  --path dist/1.0.0+1/hello_world-1.0.0+1-android.apk \
  --targets pgyer
```

## Optional Parameters

You can provide additional parameters to customize your upload:

- `--pgyer-oversea`: Upload acceleration (1=overseas, 2=domestic, empty=auto-detect)
- `--pgyer-install-type`: Installation type (1=public, 2=password, 3=invite, default: 1)
- `--pgyer-password`: App installation password (required for password installation)
- `--pgyer-description`: Application description (optional)
- `--pgyer-update-description`: Version update description (optional)
- `--pgyer-install-date`: Installation validity (1=set time, 2=permanent, optional)
- `--pgyer-install-start-date`: Installation validity start date (e.g., 2018-01-01)
- `--pgyer-install-end-date`: Installation validity end date (e.g., 2018-12-31)
- `--pgyer-channel-shortcut`: Channel shortcut for update (e.g., abcd)
