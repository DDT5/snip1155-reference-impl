# Secret Contract Integration Tests

To run tests
```sh
npm --prefix tests/ install
```
or 
```sh
cd tests; npm install
```

Install node.js. I suggest using `nvm`: https://github.com/nvm-sh/nvm. Then:
```sh
nvm install node 
nvm use 16 # the latest (version 18) doesn't seem to work
```

To run tests, start `localsecret` on one terminal (note you need to set up Docker)...
```sh
make start-server
```
...then, on another terminal:
```sh
make integration-test
```
Note that sometimes it throws an error if done too quicky in succession (exit without headers). Just run `make integration-test` again.

To run using the js debug terminal (on VS Code):
1. Press `ctrl+shift+p`
2. Write `JavaScript Debug Terminal` and press `Enter`
3. In the new terminal you can run `make integration-test`
4. Your code will be running in debug mode and will stop on every breakpoint placed.

To increase block speed:
```sh
make speedup-server
```

Don't forget to stop localsecret, which can continue running in the background
```sh
make stop-server
docker ps    # to check if the container is still running
```

To add tsconfig.json file (although not strictly necessary to run the tests), go the `./tests` directory, then:
```sh
npx tsc --init 
```
