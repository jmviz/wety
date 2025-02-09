# My Next.js App

This is a basic Next.js application that demonstrates how to fetch JSON data and render it as a tree structure in HTML.

## Project Structure

```
my-nextjs-app
├── pages
│   ├── api
│   │   └── hello.js        # API endpoint that returns a greeting message
│   ├── _app.js             # Custom App component for global styles and state
│   └── index.js            # Main entry point for the application
├── public
│   └── test
│       └── pie2.json       # JSON data used in the application
├── src
│   ├── scripts
│   │   └── script.js       # Script to generate HTML from JSON data
│   └── styles
│       └── Home.module.css  # CSS styles for the Home component
├── package.json             # npm configuration file
├── next.config.js           # Next.js configuration settings
└── README.md                # Project documentation
```

## Getting Started

To get started with this project, follow these steps:

1. **Clone the repository:**
   ```
   git clone <repository-url>
   cd my-nextjs-app
   ```

2. **Install dependencies:**
   ```
   npm install
   ```

3. **Run the development server:**
   ```
   npm run dev
   ```

4. **Open your browser and navigate to:**
   ```
   http://localhost:3000
   ```

## Usage

- The main page (`index.js`) fetches data from `public/test/pie2.json` and displays it in a tree structure.
- The API endpoint (`api/hello.js`) can be accessed at `/api/hello` to receive a greeting message in JSON format.

## Contributing

Feel free to submit issues or pull requests to improve this project.