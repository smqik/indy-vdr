package vdr

/*
#cgo LDFLAGS: -lindy_vdr
#include "libindy_vdr.h"
#include <stdlib.h>

extern void submitRequest(ErrorCode err, const char* response);
typedef void (*submitRequestWrapper)(ErrorCode err, const char* response);

extern void refresh(ErrorCode err);
typedef void (*refreshWrapper)(ErrorCode err);

extern void status(ErrorCode err, const char* response);
typedef void (*statusWrapper)(ErrorCode err, const char* response);

*/
import "C"

import (
	"fmt"
	"io"
)

func InitPool(genesis io.ReadCloser) (*Client, error) {
	newVDRClient, err := New(genesis)
	if err != nil {
		return nil, err
	}
	return newVDRClient, nil
}

func (r *Client) CreateWallet(key string, walletname string) {
	fmt.Println("Wallet create function executing")
	keyptr := C.CString(key)
	walletptr := C.CString(walletname)
	fmt.Println("function is running!!!!")
	C.wallet_create(keyptr, walletptr)
	fmt.Println("function is executed!!! ")
}

func (r *Client) OpenWallet(key string, walletname string) {
	keyptr := C.CString(key)
	walletptr := C.CString(walletname)
	wallet := C.wallet_open(keyptr, walletptr)
	fmt.Println("function is executed!!! ", wallet)
}

func (r *Client) DIDCreate(key string, walletname string, seed string, didMethod string, metadata string) {
	fmt.Println("Wallet create function executing")
	keyptr := C.CString(key)
	walletptr := C.CString(walletname)
	// seedBytes, err := identifiers.ConvertSeed(seed[0:32])
	// if err != nil {
	// 	log.Fatalln(err)
	// }
	seedptr := C.CString(seed[0:32])
	methodptr := C.CString(didMethod)
	metadataptr := C.CString(metadata)
	d := C.did_create(keyptr, walletptr, seedptr, methodptr, metadataptr)
	fmt.Println("function is executed!!! ", d)
}

func (r *Client) ListDids(key string, walletname string) {
	keyptr := C.CString(key)
	walletptr := C.CString(walletname)
	wallet := C.did_list(keyptr, walletptr)
	fmt.Println("function is executed!!! ", wallet)
}
