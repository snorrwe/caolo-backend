CREATE TABLE world_map (
    q INTEGER NOT NULL,
    r INTEGER NOT NULL,
    payload JSON NOT NULL,
    created TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    PRIMARY KEY (q,r)
);
