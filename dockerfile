FROM golang:latest AS build

RUN apt-get update
RUN apt-get install capnproto -y

WORKDIR /caolo

COPY . .

RUN go build

# ---------- Copy the built binary to a scratch container, to minimize the image size ----------

FROM ubuntu:18.04
WORKDIR /caolo
RUN apt-get update -y
RUN apt-get install curl libpq-dev -y --fix-missing

COPY --from=build /caolo/caolo-backend ./

RUN ls -al /caolo

ENTRYPOINT ["./caolo-backend"]
