// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2021 Google LLC
//
// SPDX-License-Identifier: Apache-2.0

/// Package main provides an FFI API for the trillian board to be exported and
/// used by the bulletin board rust code.
///
/// For information about C GO FFI API:
/// - https://pkg.go.dev/cmd/cgo
/// - https://github.com/golang/go/wiki/cgo#turning-c-arrays-into-go-slices
package main

/*
// #include <stdio.h>
// #include <stdlib.h>
#include <stdint.h>

struct GenerateKeysResult {
    char* publicKey;
    char* privateKey;
    char* errorCode;
	char* errorDescription;
};

struct IntegrateResult {
	char* checkpointOrigin;
	uint64_t checkpointSize;
	char* checkpointHash;
	char* errorCode;
	char* errorDescription;
};

struct Entry {
	char* tempFilePath;
	unsigned char hash[32];
};

struct SequenceResult {
	uint64_t* entriesSequenceIds;
	char* errorCode;
	char* errorDescription;
};

struct ReadCheckpointResult {
	char* checkpointOrigin;
	uint64_t checkpointSize;
	char* checkpointHash;
	char* errorCode;
	char* errorDescription;
};
*/
import "C"

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"errors"
	"flag"
	"fmt"
	"unsafe"

	"github.com/google/trillian-examples/serverless/pkg/log"
	"github.com/sequentech/bulletin-board/trillian-board/storage/fs"
	"github.com/transparency-dev/merkle/rfc6962"
	"golang.org/x/mod/sumdb/note"

	fmtlog "github.com/google/trillian-examples/formats/log"
)

//export CGenerateKeys
func CGenerateKeys(name *C.char) *C.struct_GenerateKeysResult {
	keyName := C.GoString(name)
	result := (*C.struct_GenerateKeysResult)(C.malloc(C.size_t(unsafe.Sizeof(C.struct_GenerateKeysResult{}))))

	// initialize to zero all fields
	result.privateKey = nil
	result.publicKey = nil
	result.errorCode = nil
	result.errorDescription = nil

	if len(keyName) == 0 {
		result.errorCode = C.CString("NameEmpty")
		result.errorDescription = C.CString("")
		return result
	}

	privateKey, publicKey, err := note.GenerateKey(rand.Reader, keyName)
	if err != nil {
		result.errorCode = C.CString("KeyGenerationError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	result.privateKey = C.CString(privateKey)
	result.publicKey = C.CString(publicKey)

	return result
}

//export CIntegrate
func CIntegrate(
	// Root directory to store log data
	storageDirC *C.char,
	// Set when creating a new log to initialise the structure
	initialise bool,
	// Public key
	pubKeyC *C.char,
	// Private key
	privKeyC *C.char,
	// Log origin string to use in produced checkpoint.
	originC *C.char,
) *C.struct_IntegrateResult {
	// prevents this kind of error https://github.com/sigstore/rekor/issues/68
	_ = flag.CommandLine.Parse([]string{})
	storageDir := C.GoString(storageDirC)
	pubKey := C.GoString(pubKeyC)
	privKey := C.GoString(privKeyC)
	origin := C.GoString(originC)

	result := (*C.struct_IntegrateResult)(C.malloc(C.size_t(unsafe.Sizeof(C.struct_IntegrateResult{}))))

	// initialize to zero all fields
	result.checkpointOrigin = nil
	result.checkpointSize = 0
	result.checkpointHash = nil
	result.errorCode = nil
	result.errorDescription = nil

	ctx := context.Background()

	if len(origin) == 0 {
		result.errorCode = C.CString("OriginEmpty")
		result.errorDescription = C.CString("")
		return result
	}

	h := rfc6962.DefaultHasher

	var cpNote note.Note
	s, err := note.NewSigner(privKey)
	if err != nil {
		result.errorCode = C.CString("SignerInstantiationError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	if initialise {
		st, err := fs.Create(storageDir)
		if err != nil {
			result.errorCode = C.CString("LogCreationError")
			result.errorDescription = C.CString(err.Error())
			return result
		}
		cp := fmtlog.Checkpoint{
			Hash: h.EmptyRoot(),
		}
		if err := signAndWrite(ctx, &cp, cpNote, s, st, origin); err != nil {
			result.errorCode = C.CString("CheckpointSigningError")
			result.errorDescription = C.CString(err.Error())
			return result
		}
		result.checkpointOrigin = C.CString(cp.Origin)
		result.checkpointSize = (C.ulong)(cp.Size)
		result.checkpointHash = C.CString(base64.StdEncoding.EncodeToString(cp.Hash))
		return result
	}

	// init storage
	cpRaw, err := fs.ReadCheckpoint(storageDir)
	if err != nil {
		result.errorCode = C.CString("ReadLogCheckpointError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	// Check signatures
	v, err := note.NewVerifier(pubKey)
	if err != nil {
		result.errorCode = C.CString("VerifierInstantiationError")
		result.errorDescription = C.CString(err.Error())
		return result
	}
	cp, _, _, err := fmtlog.ParseCheckpoint(cpRaw, origin, v)
	if err != nil {
		result.errorCode = C.CString("CheckpointParsingError")
		result.errorDescription = C.CString(err.Error())
		return result
	}
	st, err := fs.Load(storageDir, cp.Size)
	if err != nil {
		result.errorCode = C.CString("StorageLoadError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	// Integrate new entries
	newCp, err := log.Integrate(ctx, *cp, st, h)
	if err != nil {
		result.errorCode = C.CString("IntegrationError")
		result.errorDescription = C.CString(err.Error())
		return result
	}
	if newCp == nil {
		return result
	}

	err = signAndWrite(ctx, newCp, cpNote, s, st, origin)
	if err != nil {
		result.errorCode = C.CString("CheckpointSigningError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	result.checkpointOrigin = C.CString(newCp.Origin)
	result.checkpointSize = (C.ulong)(newCp.Size)
	result.checkpointHash = C.CString(base64.StdEncoding.EncodeToString(newCp.Hash))
	return result
}

func signAndWrite(
	ctx context.Context,
	cp *fmtlog.Checkpoint,
	cpNote note.Note,
	s note.Signer,
	st *fs.Storage,
	origin string,
) error {
	cp.Origin = origin
	cpNote.Text = string(cp.Marshal())
	cpNoteSigned, err := note.Sign(&cpNote, s)
	if err != nil {
		return fmt.Errorf("failed to sign Checkpoint: %w", err)
	}
	if err := st.WriteCheckpoint(ctx, cpNoteSigned); err != nil {
		return fmt.Errorf("failed to store new log checkpoint: %w", err)
	}
	return nil
}

//export CSequence
func CSequence(
	// Root directory to store log data
	storageDirC *C.char,
	// List of entries to sequence
	entriesC *C.struct_Entry,
	// Number of entries
	numEntries C.size_t,
	// Public key
	pubKeyC *C.char,
	// Log origin string to use in produced checkpoint.
	originC *C.char,
) *C.struct_SequenceResult {
	// prevents this kind of error https://github.com/sigstore/rekor/issues/68
	_ = flag.CommandLine.Parse([]string{})
	storageDir := C.GoString(storageDirC)
	pubKey := C.GoString(pubKeyC)
	origin := C.GoString(originC)
	entries := unsafe.Slice(entriesC, numEntries)

	result := (*C.struct_SequenceResult)(C.malloc(C.size_t(unsafe.Sizeof(C.struct_SequenceResult{}))))

	// initialize to zero all fields
	result.entriesSequenceIds = (*C.ulong)(C.malloc(C.size_t(C.sizeof_ulong * numEntries)))
	result.errorCode = nil
	result.errorDescription = nil

	// access the entriesSequenceIds as a slice and set it to all zeros
	entriesSequenceIdsSlice := unsafe.Slice(result.entriesSequenceIds, numEntries)
	for i := C.ulong(0); i < numEntries; i++ {
		entriesSequenceIdsSlice[0] = C.ulong(0)
	}

	// init storage
	cpRaw, err := fs.ReadCheckpoint(storageDir)
	if err != nil {
		result.errorCode = C.CString("ReadLogCheckpointError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	// Check signatures
	v, err := note.NewVerifier(pubKey)
	if err != nil {
		result.errorCode = C.CString("VerifierInstantiationError")
		result.errorDescription = C.CString(err.Error())
		return result
	}
	cp, _, _, err := fmtlog.ParseCheckpoint(cpRaw, origin, v)
	if err != nil {
		result.errorCode = C.CString("CheckpointParsingError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	st, err := fs.Load(storageDir, cp.Size)
	if err != nil {
		result.errorCode = C.CString("StorageLoadError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	for entryIndex, entry := range entries {
		// ask storage to sequence
		entryHash := C.GoBytes(unsafe.Pointer(&entry.hash), 32)
		tempFilePath := C.GoString(entry.tempFilePath)
		sequenceId, err := st.SequenceTempFile(
			context.Background(),
			entryHash,
			tempFilePath,
		)
		if err != nil {
			if errors.Is(err, log.ErrDupeLeaf) {
				result.errorCode = C.CString("DuplicatedEntryError")
				result.errorDescription = C.CString(err.Error())
				return result
			} else {
				result.errorCode = C.CString("EntrySequencingError")
				result.errorDescription = C.CString(err.Error())
				return result
			}
		}
		entriesSequenceIdsSlice[entryIndex] = C.ulong(sequenceId)
	}

	return result
}

//export CReadCheckpoint
func CReadCheckpoint(
	// Root directory to store log data
	storageDirC *C.char,
	// Public key
	pubKeyC *C.char,
	// Log origin string
	originC *C.char,
) *C.struct_ReadCheckpointResult {
	// prevents this kind of error https://github.com/sigstore/rekor/issues/68
	_ = flag.CommandLine.Parse([]string{})
	storageDir := C.GoString(storageDirC)
	pubKey := C.GoString(pubKeyC)
	origin := C.GoString(originC)

	result := (*C.struct_ReadCheckpointResult)(C.malloc(C.size_t(unsafe.Sizeof(C.struct_ReadCheckpointResult{}))))

	// initialize to zero all fields
	result.checkpointOrigin = nil
	result.checkpointSize = 0
	result.checkpointHash = nil
	result.errorCode = nil
	result.errorDescription = nil

	// init storage
	cpRaw, err := fs.ReadCheckpoint(storageDir)
	if err != nil {
		result.errorCode = C.CString("ReadLogCheckpointError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	// Check signatures
	v, err := note.NewVerifier(pubKey)
	if err != nil {
		result.errorCode = C.CString("VerifierInstantiationError")
		result.errorDescription = C.CString(err.Error())
		return result
	}
	cp, _, _, err := fmtlog.ParseCheckpoint(cpRaw, origin, v)
	if err != nil {
		result.errorCode = C.CString("CheckpointParsingError")
		result.errorDescription = C.CString(err.Error())
		return result
	}

	result.checkpointOrigin = C.CString(cp.Origin)
	result.checkpointSize = (C.ulong)(cp.Size)
	result.checkpointHash = C.CString(base64.StdEncoding.EncodeToString(cp.Hash))
	return result
}

func main() {}
