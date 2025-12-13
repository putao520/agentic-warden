# Google OAuth Setup for AIW

## Configuration Options

AIW provides two ways to configure Google OAuth credentials:

### Option 1: Configuration File (Recommended)

Create a configuration file at `~/.aiw/oauth_config.json`:

```json
{
  "client_id": "your-client-id.apps.googleusercontent.com",
  "client_secret": "your-client-secret",
  "scopes": [
    "https://www.googleapis.com/auth/drive.file",
    "https://www.googleapis.com/auth/drive.metadata.readonly"
  ]
}
```

### Option 2: Use Built-in Public Client (For Testing Only)

AIW comes with a built-in public OAuth client that works for testing:

```bash
aiw push
```

However, this public client has limitations:
- May encounter rate limits
- Not suitable for production use
- Could be disabled without notice

## Creating Your Own Google OAuth Credentials

For the best experience, create your own Google OAuth credentials:

### Step 1: Go to Google Cloud Console
1. Visit [https://console.cloud.google.com/](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Make sure the Google Drive API is enabled for your project

### Step 2: Configure OAuth Consent Screen
1. Go to **APIs & Services** > **OAuth consent screen**
2. Choose **External** user type
3. Fill in required information:
   - App name: "AIW" or your preferred name
   - User support email: your email
   - Developer contact information: your email
4. Click **Save and Continue**
5. Add test users if needed (for external type)
6. Click **Save and Continue**

### Step 3: Create OAuth Client ID
1. Go to **APIs & Services** > **Credentials**
2. Click **Create Credentials** > **OAuth client ID**
3. Choose **Desktop app** as application type
4. Give it a name (e.g., "AIW Desktop Client")
5. Click **Create**

### Step 4: Configure Client ID
After creating the OAuth client ID:
1. Note down the **Client ID** and **Client Secret**
2. Click on the newly created client ID to edit it
3. Under "Authorized redirect URIs", add:
   ```
   http://localhost:8080/oauth/callback
   ```
   (This is required even though device flow doesn't use redirects)
4. Save your changes

### Step 5: Use Your Credentials

Configure AIW using the configuration file method (Option 1 above).

## Troubleshooting

### Common Error: "invalid_client"

This error usually means:
1. Your Client ID or Client Secret is incorrect
2. The OAuth client type doesn't support device flow
3. The Google Drive API is not enabled
4. Your OAuth consent screen is not published

**Solutions:**
1. Double-check your Client ID and Client Secret
2. Try creating a new OAuth client ID
3. Ensure the Google Drive API is enabled
4. Publish your OAuth consent screen

### Common Error: "unauthorized_client"

This means the client ID is not authorized for device flow. Make sure:
1. You're using a "Desktop app" or "Web application" client type
2. The client ID is properly configured with redirect URIs
3. Your OAuth consent screen is published

### Using the Public Client

If you prefer to use the built-in public client:

```bash
aiw push
```

You'll see a warning message indicating this is a public client with potential limitations.

## Best Practices

1. **For Production**: Always create your own OAuth credentials
2. **For Testing**: The public client works for basic testing
3. **For Security**: Use environment variables or secure config files
4. **For Reliability**: Avoid relying on public clients for critical applications

## Support

If you continue to have issues:
1. Check the [Google OAuth Documentation](https://developers.google.com/identity/protocols/oauth2)
2. Review [Google Drive API Setup](https://developers.google.com/drive/api)
3. Consider creating a new OAuth client ID from scratch