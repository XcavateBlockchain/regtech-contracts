# Company Owner Documentation

**RegTech Credential Platform**

A practical guide to setting up your organization, publishing compliance modules, managing your team, and issuing on-chain credentials.

---

**Version:** 1.0
**Audience:** Organization Owners
**Network:** Solana Devnet

---

## Contents

1. [Before you begin](#01-before-you-begin)
2. [Create your account](#02-create-your-account)
3. [Set up your organization](#03-set-up-your-organization)
4. [Fund your wallet and allocate quizzes](#04-fund-your-wallet-and-allocate-quizzes)
5. [Create a compliance module](#05-create-a-compliance-module)
6. [Publish your module](#06-publish-your-module)
7. [Invite your employees](#07-invite-your-employees)
8. [Share modules with investors](#08-share-modules-with-investors)
9. [Monitor results](#09-monitor-results)
10. [Manage credentials](#10-manage-credentials)
11. [Company settings](#11-company-settings)
12. [Quick reference](#12-quick-reference)

---

## 01 Before you begin

This guide walks you through everything you need to run your organization on the platform, from signing up to issuing your first credentials. You don't need any prior blockchain experience. Where on-chain steps happen, we handle the heavy lifting and tell you what's going on.

Before you create an account, make sure you have one of the following:

- A Google account. We'll create a Solana wallet for you behind the scenes, so you won't need to manage private keys.
- An existing Solana wallet such as Phantom, if you'd prefer to use one you already control.

The platform currently runs on Solana Devnet, so no real funds are involved at any point. Anything that looks like a transaction is happening on a test network.

> **Good to know**
> Each account can only have one role: Owner, Employee, or Investor. If you've already used a Google account or wallet to join as an employee or investor, you'll need a different one to register as a company owner.

---

## 02 Create your account

Head to the landing page and click **Get Started**. You'll see two sign-in options:

- **Continue with Google (wallet).** The fastest path, ideal for signing into an existing account.
- **Continue with wallet (advanced).** Required for creating an organization. This is the option labeled "For organization only."

![Landing page sign-in modal showing the two options](screenshots/image1.png)

Pick **Continue with wallet (advanced)**. A Phantom-powered screen will open with two options:

- **Continue with Google.** Sign in with your Google account. Phantom creates and manages a wallet for you behind the scenes.
- **Other Wallets.** Connect an existing Solana wallet like Phantom, Backpack, or any other compatible wallet.

![Phantom wallet connection screen with the two options](screenshots/image2.png)

Either path works. Once you authenticate, the platform takes you straight into the organization setup flow described next.

> **Why the wallet path**
> Organizations are tied to a wallet because the company vault that funds your modules is created on-chain. The first sign-in option on the platform ("Continue with Google (wallet)") is for individuals joining as employees or investors, and it doesn't support owning an organization.
>
> The "wallet (advanced)" path routes through Phantom's full sign-in flow, which is required for company registration.

---

## 03 Set up your organization

Setup happens in three short steps. None of these are permanent; you can edit most of this later from Settings.

### Your profile

Enter your name and email. Your wallet address fills in automatically.

![Profile setup form, step 1 of 3](screenshots/image3.png)

| Field | What to enter |
|---|---|
| Name | Your full name as you'd like it shown to your team. |
| Email | A working contact address. Used for account recovery and notifications. |
| Wallet address | Auto-filled from the wallet you just connected. Read-only. |

### Company details

This is the public-facing identity of your organization on the platform.

![Company details form, step 2 of 3](screenshots/image4.png)

| Field | Details |
|---|---|
| Company name | How your organization appears on dashboards, modules, and credentials. |
| Company URL | A short, unique slug for your dashboard, e.g. `acme-corp`. You can change it later. |
| Industry | Pick from Real Estate, Marketplace, DeFi, or Other. |
| Description | Optional. A line or two about what your organization does, useful for participants who see your modules. |

When you're happy with the form, click **Create account**.

### On-chain setup

The platform now creates your company's on-chain wallet, called a Swig vault. This is the account that pays for credential minting and other on-chain operations later. It happens automatically; you'll see a brief loading state and then land on your company dashboard.

![On-chain setup loading state, step 3 of 3](screenshots/image5.png)

> **What's a Swig vault?**
> Think of it as your company's on-chain identity. It signs transactions on behalf of your organization, including publishing modules and issuing credentials. Alongside it, a Partner account tracks your quiz capacity and holds the SOL balance used to cover on-chain program costs.
>
> You don't need to manage either of these directly. The dashboard handles everything for you. When you allocate quizzes or request funding, you're updating these on-chain accounts behind the scenes.

---

## 04 Fund your wallet and allocate quizzes

Before you can publish modules and issue credentials, you need two things in place:

- Quiz capacity. The number of attempts your participants can take collectively.
- A small SOL balance in your vault, used to cover on-chain transaction fees.

Both are managed from **Settings**, in the **Company Vault** section.

![Settings page with Company Vault section visible](screenshots/image6.png)

### Allocate quizzes

Open **Settings** in the navigation bar and scroll to **Company Vault**. You'll see your current quiz balance, which is how many attempts you have available across all your modules.

1. Click **Allocate quizzes** to add 100 slots at a time.
2. Each module attempt by an employee or investor consumes one slot.
3. If you have unused slots, enter the number you want to release and click **Refund** to reclaim them.

![Allocate quizzes modal with slot input field](screenshots/image7.png)

> **Heads up**
> When your remaining quiz balance drops below 10, the platform surfaces a warning on the module creation page. You'll also see alerts if your balance hits zero or doesn't cover the number of recipients you've configured for a module.

### Request vault funding

Your vault needs SOL to cover the small transaction fees that come with on-chain operations like minting credentials. Funding is handled as a request that a platform administrator reviews.

1. Under **Request vault funding**, enter the amount of SOL you need.
2. Optionally set a daily cap to limit how much can be spent per day, and add a short note explaining the request. This helps the reviewer respond quickly.
3. Click **Submit funding request**.

You can track the status of every request right from the same page:

| Status | What it means |
|---|---|
| Pending | Submitted and awaiting review. |
| Approved | Funds have been transferred to your vault. |
| Rejected | Reviewer declined the request. The note will explain why. |
| Cancelled | You withdrew the request before it was reviewed. |

Pending requests can be cancelled at any time from the requests list.

---

## 05 Create a compliance module

Modules are the core of the platform. Each one is a structured assessment your employees and investors complete to earn a verified credential.

### Start a new module

Open **Modules** in the navigation bar and click **Create Module**. You'll work through three groups of settings: details, quiz configuration, and learning materials.

![Modules page with Create Module button highlighted](screenshots/image8.png)

### Module details

![Module details form section](screenshots/image9.png)

| Field | Details |
|---|---|
| Name | What participants see in their list. Be specific. "FCA Investment Compliance Q2 2026" beats "Compliance Module". |
| Description | A short summary of what the module covers and who it's for. |
| Module type | The regulatory framework: FCA Investment, FCA Regulated, or SEC Framework. This determines which curated question bank is used. |
| Category | The topic area: Securities, Anti-Money Laundering, KYC, DeFi, or Tax & Reporting. |
| Language | English, Spanish, French, German, or Chinese. |
| Est. completion time | Pick 15, 30, 45, 60, or 90 minutes. This is the figure participants see before starting. |
| Thumbnail | An image that represents the module on cards and listings. |

### Quiz settings

These settings control how the assessment behaves. Two of them lock once you publish, so review them carefully before going live.

![Quiz settings form section](screenshots/image10.png)

| Setting | Details |
|---|---|
| Passing score | The minimum percentage to pass. 80% is a sensible default for compliance content. |
| Quiz time limit | How long participants have to complete the quiz. Set to 0 for no limit. |
| Retry cooldown | How long someone must wait before retrying after a failed attempt. Default is 24 hours. **Locked after publish.** |
| Credential expiry | How many months credentials remain valid. Set to 0 for no expiry. **Locked after publish.** |
| Number of recipients | The cap on how many participants can complete this module. Each recipient costs $1 USD. |

> **Why some settings lock**
> Retry cooldown and credential expiry are written on-chain when the module is published. Changing them later would invalidate the integrity of credentials already issued, so we don't allow it. If you need to change them, create a new module with the updated settings.

### Questions

Questions are loaded automatically from a curated bank tied to the module type you selected. Each module includes multiple quiz variations (batches), and each participant is randomly assigned one batch when they start an attempt. All batches are calibrated to have equal total points, so no matter which variation a participant gets, the difficulty is consistent.

### Learning materials (optional)

Under **Module Contents** you can upload supplementary files (PDFs, slide decks, internal documents) that participants can review before or during the assessment. Files are stored securely and only visible to enrolled participants.

![Module contents upload area](screenshots/image11.png)

---

## 06 Publish your module

New modules start in **Draft** status. While in draft only you can see them: they aren't visible to participants and no one can enroll.

When the module is ready to go live:

1. Open the module from your **Modules** list.
2. Click **Publish**.

![Module detail page with Publish button visible](screenshots/image12.png)

The module's status changes to **Active**. From this point on, the module is registered on-chain, visible to invited employees and anyone with a share link, and accepting enrollments.

> **Once published, a module stays active**
> There is currently no way to archive or delete a published module from the dashboard. On-chain records are permanent by design, and credentials already issued remain valid regardless of any future changes.

If you need to make a substantive change (anything that affects scoring or eligibility), create a new module with the updated settings. Existing credentials from the original module keep their original terms.

---

## 07 Invite your employees

Employees are the members of your organization with internal permissions. They can't sign themselves up; you invite them by email.

### Send an invite

1. Open **Team** in the navigation bar.
2. Click **Add Employee**.
3. Enter the employee's name and email, then choose a permission level.

![Add Employee form with name, email, and role fields](screenshots/image13.png)

The available roles and what they can do:

| Role | What they can do |
|---|---|
| Reviewer | View assessments and learner results. Cannot edit modules or issue credentials. |
| Issuer | Mint and revoke credentials in addition to reviewing results. |
| Auditor | Read-only access across the entire organization. Useful for compliance officers and external auditors. |

4. Submit the form. The platform generates an invite link.
5. Copy the link and send it to the employee through your usual channel: email, Slack, or anything else.

![Generated invite link with copy button](screenshots/image14.png)

### What the employee sees

Your employee opens the invite link, signs in with their Google account, confirms the details on screen, and clicks **Claim invite**. They're now part of your team, but they won't see any modules yet. You need to assign modules to them first (see below).

![Employee-facing claim invite page](screenshots/image15.png)

### Assign modules to employees

After an employee has joined your team, you need to assign them to the modules they should complete.

1. Open **Modules** in the navigation bar.
2. Click into the published module you want to assign.
3. In the assignment panel, select the employees you want to assign.
4. Confirm the assignment.

![Module assignment panel with employee selection list](screenshots/image16.png)

Each assigned employee is enrolled on-chain and the module appears on their dashboard with a **Pending** status, ready to start.

### Manage your team

From the **Team** page you can:

- See all active employees and their assigned roles.
- View pending invitations that haven't been claimed yet.
- Re-share an invite link if the original was lost or never reached the recipient.

---

## 08 Share modules with investors

Investors are external participants: clients, partners, or third parties who need to complete one or more of your compliance modules but aren't part of your internal team.

Unlike employees, investors don't need an explicit invite. You share a link, they join.

### Get the share link

1. Open **Modules** in the navigation bar.
2. Find the published module you want to share and click into it.
3. Copy the share link from the module page.

![Module detail page with share link area highlighted](screenshots/image17.png)

### How investors join

Anyone with the link can join the module. They'll see your company name and the module description, sign in with Google (or connect a wallet), enter their name and email, and click **Join and continue**. They're enrolled immediately and can start the assessment.

![Investor-facing module join page](screenshots/image18.png)

> **One link per module**
> Each share link is tied to a single module. If you need investors to complete more than one, send them a separate link for each. This keeps each module's enrollment list clean and auditable.

Investors only have access to the modules you share with them. They can't see your dashboard, your team, or anything else about your organization.

---

## 09 Monitor results

### Dashboard overview

Your company dashboard, at `/<your-slug>`, gives you the headline numbers at a glance:

![Company dashboard with stats cards and activity feed](screenshots/image19.png)

| Metric | What it shows |
|---|---|
| Employees | Number of employees in your organization. |
| Active Modules | Modules currently published and accepting enrollments. |
| Credentials issued | Total credentials minted on-chain to date. |
| Public enrolled | External participants (investors) who have joined via share links. |
| Total enrolments | Combined count of all enrollments across your modules. |

### Activity

The dashboard displays your recent activity feed: a chronological log of events across your organization, including modules created, employees invited, credentials issued, and enrollments.

![Recent activity feed with several event entries](screenshots/image19.png)

### Module performance

Click into any module from the dashboard or Modules list to see its detail page, including:

![Module detail page with performance metrics](screenshots/image20.png)

| Metric | What it shows |
|---|---|
| Total enrolments | Number of participants who have joined the module. |
| Passed users | Percentage of participants who met the passing score. |
| Failed user | Percentage of participants who did not pass. |

### Learner results

Each module's detail page includes a learner-by-learner breakdown: who attempted the quiz and when, the score they achieved, their pass/fail status, and how many attempts they've used. Use this to follow up with anyone struggling, or to surface evidence of completion for an external audit.

![Learner results table for a module](screenshots/image21.png)

---

## 10 Manage credentials

When a participant passes a module, the platform issues them a credential: a tamper-proof, on-chain proof of completion that anyone can verify on the Solana blockchain.

From the **Credentials issued** section on your dashboard, you can view every credential your organization has issued in a table showing:

![Credentials issued table with several entries](screenshots/image22.png)

| Column | Details |
|---|---|
| Recipient | The wallet address of the participant who earned it. |
| Module | The module that was completed. |
| Score | The achieved score at the time of issuance. |
| NFT | Link to view the credential NFT on-chain. |
| Tx | Link to the minting transaction on the Solana explorer. |
| Issued | The date and time the credential was minted. |

> **Verification is public**
> Anyone with the credential's asset address can verify it independently on Solana. They don't need an account on the platform, and they don't need to ask you. That's the point: credentials carry their own proof of authenticity.

---

## 11 Company settings

Settings is where you manage your organization's public-facing details and on-chain wallet. Open it from the navigation bar.

### Company profile

Update any of the following at any time and click **Save changes**:

- Display name and URL slug.
- Description, logo, and website link.
- Owner contact email.

Click **Save changes** to apply your updates.

### Company wallet

This section consolidates everything related to your on-chain operations:

- **Vault address.** The on-chain identity of your company wallet, useful for verification or external integrations.
- **SOL balance.** The current funding level for transaction fees.
- **Quiz allocation.** Current capacity and history of allocations and refunds.
- **Funding requests.** Full history of requests with their statuses.
- **Transaction history.** Every on-chain operation tied to your organization, with timestamps and links to the Solana explorer.

Everything covered in [Section 04](#04-fund-your-wallet-and-allocate-quizzes) lives here. That section walks through the actions; this is the dashboard view of where they end up.

---

## 12 Quick reference

If you've already worked through the guide once, here's the condensed flow for the next time you set up a module.

```
1.  Sign up      >  Get Started > Continue with wallet (advanced)
2.  Set up       >  Enter your profile and company details
3.  Fund         >  Settings > Allocate quizzes + request vault funding
4.  Create       >  Modules > Create Module > details + quiz settings
5.  Publish      >  Open the draft module > Publish
6.  Invite       >  Team > Add Employee > share invite link
7.  Share        >  Modules > copy share link > send to investors
8.  Monitor      >  Dashboard > stats, activity, and learner results
9.  Manage       >  Track credentials from the dashboard
```

---

*End of document*