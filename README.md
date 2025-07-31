# pdf-api

Very simple API for generating PDF files from HTML.

### Usage

I don't recommend exposing this API to the internet as I don't do any validation before sending the HTML to the headless Chrome browser, but you can run it locally, or in a private network.

```shell
docker run -p 3000:3000 --rm --name pdf-api ghcr.io/borisnliscool/pdf-api
```

_or build it yourself_

```shell
docker build -t pdf-api .
docker run -p 3000:3000 --rm --name pdf-api pdf-api
```