-- A photo of the finished dish, or of the card the recipe was copied from.
--
-- Held inline rather than as a file on disk so that the whole application state
-- is still one file to copy or back up. Both columns are nullable together: a
-- recipe without a photo has NULL in each.
ALTER TABLE recipes ADD COLUMN photo BLOB;

-- The browser decides the encoding when it hands over a file, and iOS in
-- particular may deliver HEIC rather than JPEG. Storing the type it reported
-- means the image can be served back with a header that matches the bytes,
-- instead of guessing at display time.
ALTER TABLE recipes ADD COLUMN photo_mime TEXT;
