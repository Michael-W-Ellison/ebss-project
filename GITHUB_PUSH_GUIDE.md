# Pushing EBSS Project to GitHub

## Option 1: Using GitHub CLI (Recommended)

If you have GitHub CLI installed:

```bash
cd /path/to/ebss-project

# Login to GitHub (if not already)
gh auth login

# Create repository and push
gh repo create ebss-project --public --source=. --remote=origin --push

# Or for private repository
gh repo create ebss-project --private --source=. --remote=origin --push
```

## Option 2: Using Git + GitHub Web Interface

### Step 1: Create Repository on GitHub
1. Go to https://github.com/new
2. Repository name: `ebss-project` (or `emergent-behavior-simulator`)
3. Description: "Emergent Behavior Society Simulator - AI platform for autonomous agent societies"
4. Choose Public or Private
5. **DO NOT** initialize with README, .gitignore, or license (we already have these)
6. Click "Create repository"

### Step 2: Push Your Local Repository

```bash
cd /path/to/ebss-project

# Add the remote (replace YOUR_USERNAME with your GitHub username)
git remote add origin https://github.com/YOUR_USERNAME/ebss-project.git

# Verify the remote was added
git remote -v

# Push to GitHub
git branch -M main
git push -u origin main
```

### Step 3: Verify Upload
Go to `https://github.com/YOUR_USERNAME/ebss-project` and verify all files are there.

## Option 3: Using SSH (If you have SSH keys set up)

```bash
cd /path/to/ebss-project

# Add remote using SSH
git remote add origin git@github.com:YOUR_USERNAME/ebss-project.git

# Push
git branch -M main
git push -u origin main
```

## What Gets Pushed

The following will be uploaded to GitHub:

```
✅ All source code (src/)
✅ Examples (examples/)
✅ Documentation (README.md, SETUP.md, CONTRIBUTING.md)
✅ Build configuration (Cargo.toml)
✅ CI/CD pipeline (.github/workflows/)
✅ License (LICENSE)
✅ Git configuration (.gitignore)
```

## After Pushing

### Enable GitHub Actions
1. Go to your repository on GitHub
2. Click "Actions" tab
3. Click "I understand my workflows, go ahead and enable them"
4. Your CI/CD pipeline will start running automatically

### Add Repository Topics
Add these topics to help people find your project:
- rust
- ai
- simulation
- multi-agent
- emergent-behavior
- game-ai
- reinforcement-learning
- behavior-trees

### Set Up GitHub Pages (Optional)
For documentation hosting:
1. Go to Settings → Pages
2. Source: Deploy from a branch
3. Branch: main / docs
4. Save

## Troubleshooting

### "Permission denied (publickey)"
Set up SSH keys or use HTTPS with personal access token:
```bash
git remote set-url origin https://YOUR_USERNAME@github.com/YOUR_USERNAME/ebss-project.git
```

### "Repository not found"
Make sure the repository exists on GitHub and the URL is correct.

### Authentication Failed
Create a Personal Access Token:
1. GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Generate new token
3. Select scopes: repo (all)
4. Use token as password when pushing

## Recommended Repository Settings

After pushing, configure:
- ✅ Add description: "AI platform for simulating autonomous agent societies with emergent behaviors"
- ✅ Add website: Link to documentation
- ✅ Enable Issues
- ✅ Enable Discussions
- ✅ Add topics (see above)
- ✅ Create `develop` branch for active development
- ✅ Set branch protection rules on `main`

## Next Steps After Push

1. **Verify CI/CD**: Check that GitHub Actions runs successfully
2. **Create First Issue**: Plan Phase 1 completion tasks
3. **Add Project Board**: Organize development tasks
4. **Invite Collaborators**: If working with others
5. **Share**: Post on relevant forums/communities

---

**Your repository is ready to push! Just follow the steps above based on your preference.**
